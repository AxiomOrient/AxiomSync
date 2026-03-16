use std::fs;
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};

use crate::catalog::request_log_uri;
use crate::config::{RETRIEVAL_BACKEND_MEMORY, RETRIEVAL_BACKEND_POLICY_MEMORY_ONLY};
use crate::error::{AxiomError, Result};
use crate::jsonl::{jsonl_all_lines_invalid, parse_jsonl_tolerant};
use crate::models::{
    BackendStatus, CommitMode, CommitResult, EmbeddingBackendStatus, IngestProfile,
    MemoryPromotionRequest, MemoryPromotionResult, QueueDiagnostics, QueueOverview,
    RUN_STATUS_SUCCESS, RepairRunRecord, RequestLogEntry, ResourceQuery, ResourceRecord,
    SessionInfo, SessionMeta,
};
use crate::om::engine::model::OmRecord;
use crate::queue_policy::default_scope_set;
use crate::session::Session;
use crate::state::schema::{INDEX_PROFILE_STAMP_KEY, RUNTIME_RESTORE_SOURCE_KEY};
use crate::uri::{AxiomUri, Scope};

use super::AxiomSync;

const SEARCH_STACK_VERSION: &str = "drr-memory-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeBootstrapPlan {
    ensure_scope_tiers: bool,
    restore_decision: RuntimeRestoreDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeRestoreDecision {
    FullReindex { reason: &'static str },
    RestoreFromState { stamp: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexRepairPlan {
    repair_type: &'static str,
    restore_source: &'static str,
    index_profile_stamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionScopePlan {
    session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromotionExecutionPlan {
    session_scope: SessionScopePlan,
    checkpoint_after_promotion: bool,
}

#[derive(Debug)]
struct ProjectionRepairPlan {
    resources: Vec<ResourceRecord>,
    om_records: Vec<OmRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionDeletionPlan {
    session_scope: SessionScopePlan,
    session_uri: AxiomUri,
    should_delete: bool,
}

pub(crate) struct RuntimeBootstrapService<'a> {
    app: &'a AxiomSync,
}

impl<'a> RuntimeBootstrapService<'a> {
    pub(crate) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(crate) fn bootstrap(&self) -> Result<()> {
        self.app.fs.initialize()?;
        self.app.ensure_default_ontology_schema()?;
        Ok(())
    }

    pub(crate) fn prepare_runtime(&self) -> Result<()> {
        self.bootstrap()?;
        let current_stamp = self.current_index_profile_stamp();
        let stored_stamp = self.app.state.get_system_value(INDEX_PROFILE_STAMP_KEY)?;
        let has_drift = self.has_index_state_drift()?;
        let restored_search_documents = if stored_stamp.as_deref() == Some(current_stamp.as_str())
            && !has_drift
        {
            self.restore_index_from_state()?
        } else {
            0
        };
        let plan = build_runtime_bootstrap_plan(
            current_stamp,
            stored_stamp,
            has_drift,
            restored_search_documents,
        );
        if plan.ensure_scope_tiers {
            self.app.ensure_scope_tiers()?;
        }
        self.execute_restore_decision(&plan.restore_decision)?;
        Ok(())
    }

    pub(crate) fn initialize(&self) -> Result<()> {
        self.prepare_runtime()
    }

    pub(crate) fn reindex_all(&self) -> Result<()> {
        let started_at = Utc::now().to_rfc3339();
        self.app.state.clear_search_index()?;
        self.app.state.clear_index_state()?;
        {
            let mut index = self
                .app
                .index
                .write()
                .map_err(|_| AxiomError::lock_poisoned("index"))?;
            index.clear();
        }
        self.app.reindex_scopes(&default_scope_set())?;

        // Re-apply resource record search projections so that mount-root namespace/kind
        // invariants are not overwritten by the generic directory indexer during the
        // filesystem walk above. The resource table is the authoritative source for these fields.
        let resources = self.app.state.list_resources(ResourceQuery::default())?;
        let om_records = self.app.state.list_om_records()?;
        self.execute_projection_repair(ProjectionRepairPlan { resources, om_records })?;

        let repair_plan = build_full_reindex_repair_plan(
            self.current_index_profile_stamp(),
            "full_reindex",
        );
        self.persist_repair_plan(repair_plan, started_at)
    }

    fn execute_projection_repair(&self, plan: ProjectionRepairPlan) -> Result<()> {
        let mut index = self
            .app
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        for resource in &plan.resources {
            let uri_str = resource.uri.to_string();
            let profile = IngestProfile::for_kind(&resource.kind);
            self.app
                .state
                .persist_resource_search_document(resource, &profile)?;
            if let Some(record) = self.app.state.get_search_document(&uri_str)? {
                index.remove(&uri_str);
                index.upsert(record);
            }
        }
        for om in plan.om_records {
            index.upsert_om_record(om);
        }
        Ok(())
    }

    fn execute_restore_decision(&self, decision: &RuntimeRestoreDecision) -> Result<()> {
        match decision {
            RuntimeRestoreDecision::FullReindex { .. } => self.reindex_all(),
            RuntimeRestoreDecision::RestoreFromState { stamp } => {
                self.app
                    .state
                    .set_system_value(INDEX_PROFILE_STAMP_KEY, stamp)?;
                self.app
                    .state
                    .set_system_value(RUNTIME_RESTORE_SOURCE_KEY, "state_restore")?;
                Ok(())
            }
        }
    }

    fn persist_repair_plan(&self, repair_plan: IndexRepairPlan, started_at: String) -> Result<()> {
        self.app
            .state
            .set_system_value(INDEX_PROFILE_STAMP_KEY, &repair_plan.index_profile_stamp)?;
        self.app
            .state
            .set_system_value(RUNTIME_RESTORE_SOURCE_KEY, repair_plan.restore_source)?;
        self.app.state.record_repair_run(&RepairRunRecord {
            run_id: format!("repair-{}", uuid::Uuid::new_v4().simple()),
            repair_type: repair_plan.repair_type.to_string(),
            started_at,
            finished_at: Some(Utc::now().to_rfc3339()),
            status: RUN_STATUS_SUCCESS.to_string(),
            details: Some(serde_json::json!({
                "index_profile_stamp": repair_plan.index_profile_stamp,
            })),
        })?;
        Ok(())
    }

    fn restore_index_from_state(&self) -> Result<usize> {
        let records = self.app.state.list_search_documents()?;
        let om_records = self.app.state.list_om_records()?;
        let mut index = self
            .app
            .index
            .write()
            .map_err(|_| AxiomError::lock_poisoned("index"))?;
        index.clear();
        let restored_search_documents = records
            .into_iter()
            .filter_map(|record| {
                let uri = AxiomUri::parse(&record.uri).ok()?;
                if uri.scope().is_internal() {
                    return None;
                }
                index.upsert(record);
                Some(())
            })
            .count();
        for om in om_records {
            index.upsert_om_record(om);
        }
        drop(index);
        Ok(restored_search_documents)
    }

    fn current_index_profile_stamp(&self) -> String {
        let embed = crate::embedding::embedding_profile();
        format!(
            "stack:{};embed:{}@{}:{}",
            SEARCH_STACK_VERSION, embed.provider, embed.vector_version, embed.dim
        )
    }

    fn has_index_state_drift(&self) -> Result<bool> {
        for (uri, stored_mtime) in self.app.state.list_index_state_entries()? {
            let Ok(parsed) = AxiomUri::parse(&uri) else {
                return Ok(true);
            };
            let path = self.app.fs.resolve_uri(&parsed);
            match std::fs::metadata(&path) {
                Err(_) => return Ok(true),
                Ok(meta) => {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, saturating_duration_nanos_to_i64);
                    if mtime != stored_mtime {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

pub(crate) struct SessionService<'a> {
    app: &'a AxiomSync,
}

impl<'a> SessionService<'a> {
    pub(crate) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(crate) fn sessions(&self) -> Result<Vec<SessionInfo>> {
        let root = AxiomUri::root(Scope::Session);
        let mut sessions = self
            .app
            .fs
            .list(&root, false)?
            .into_iter()
            .filter(|e| e.is_dir)
            .map(|entry| {
                let session_uri = AxiomUri::parse(&entry.uri)?;
                Ok(SessionInfo {
                    session_id: entry.name.clone(),
                    uri: entry.uri,
                    updated_at: self.app.session_updated_at(&session_uri),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        Ok(sessions)
    }

    pub(crate) fn promote_session_memories(
        &self,
        request: &MemoryPromotionRequest,
    ) -> Result<MemoryPromotionResult> {
        let plan = build_promotion_execution_plan(request.session_id.clone(), false);
        let session = self.load_session(plan.session_scope.session_id.as_str())?;
        session.promote_memories(request)
    }

    pub(crate) fn checkpoint_session_archive_only(&self, session_id: &str) -> Result<CommitResult> {
        let plan = build_session_scope_plan(session_id);
        let session = self.load_session(plan.session_id.as_str())?;
        session.commit_with_mode(CommitMode::ArchiveOnly)
    }

    pub(crate) fn promote_and_checkpoint_archive_only(
        &self,
        request: &MemoryPromotionRequest,
    ) -> Result<MemoryPromotionResult> {
        let plan = build_promotion_execution_plan(request.session_id.clone(), true);
        let session = self.load_session(plan.session_scope.session_id.as_str())?;
        let result = session.promote_memories(request)?;
        if plan.checkpoint_after_promotion {
            let _ = session.commit_with_mode(CommitMode::ArchiveOnly)?;
        }
        Ok(result)
    }

    pub(crate) fn delete(&self, session_id: &str) -> Result<bool> {
        let session_uri = AxiomUri::root(Scope::Session).join(session_id)?;
        let plan = build_session_deletion_plan(
            session_id,
            session_uri.clone(),
            self.app.fs.exists(&session_uri),
        );
        if !plan.should_delete {
            return Ok(false);
        }
        self.app.fs.rm(&plan.session_uri, true, true)?;
        self.app.purge_uri_index(&plan.session_uri)?;
        let _ = self
            .app
            .state
            .remove_promotion_checkpoints_for_session(&plan.session_scope.session_id)?;
        Ok(true)
    }

    fn load_session(&self, session_id: &str) -> Result<Session> {
        let session = self.app.session(Some(session_id));
        session.load()?;
        Ok(session)
    }
}

impl AxiomSync {
    pub fn bootstrap(&self) -> Result<()> {
        self.runtime_bootstrap_service().bootstrap()
    }

    pub fn prepare_runtime(&self) -> Result<()> {
        self.runtime_bootstrap_service().prepare_runtime()
    }

    pub fn initialize(&self) -> Result<()> {
        self.runtime_bootstrap_service().initialize()
    }

    pub fn session(&self, session_id: Option<&str>) -> Session {
        let id = session_id.map_or_else(
            || format!("s-{}", uuid::Uuid::new_v4().simple()),
            ToString::to_string,
        );
        Session::new(id, self.fs.clone(), self.state.clone(), self.index.clone())
            .with_config(self.config.clone())
    }

    pub fn backend_status(&self) -> Result<BackendStatus> {
        let local_records = self
            .index
            .read()
            .map_err(|_| AxiomError::lock_poisoned("index"))?
            .record_count();

        let embed = crate::embedding::embedding_profile();

        Ok(BackendStatus {
            local_records,
            retrieval_backend: RETRIEVAL_BACKEND_MEMORY.to_string(),
            retrieval_backend_policy: RETRIEVAL_BACKEND_POLICY_MEMORY_ONLY.to_string(),
            embedding: EmbeddingBackendStatus {
                provider: embed.provider,
                vector_version: embed.vector_version,
                dim: embed.dim,
            },
        })
    }

    pub fn queue_diagnostics(&self) -> Result<QueueDiagnostics> {
        let queue_dead_letter_rate = self
            .state
            .queue_dead_letter_rates_by_event_type()?
            .into_iter()
            .filter(is_om_event_type)
            .collect::<Vec<_>>();
        Ok(QueueDiagnostics {
            counts: self.state.queue_counts()?,
            checkpoints: self.state.list_checkpoints()?,
            queue_dead_letter_rate,
            om_status: self.state.om_status_snapshot()?,
            om_reflection_apply_metrics: self.state.om_reflection_apply_metrics_snapshot()?,
        })
    }

    pub fn queue_overview(&self) -> Result<QueueOverview> {
        let (counts, lanes) = self.state.queue_snapshot()?;
        let queue_dead_letter_rate = self
            .state
            .queue_dead_letter_rates_by_event_type()?
            .into_iter()
            .filter(is_om_event_type)
            .collect::<Vec<_>>();
        Ok(QueueOverview {
            counts,
            checkpoints: self.state.list_checkpoints()?,
            lanes,
            queue_dead_letter_rate,
            om_status: self.state.om_status_snapshot()?,
            om_reflection_apply_metrics: self.state.om_reflection_apply_metrics_snapshot()?,
        })
    }

    pub fn list_request_logs(&self, limit: usize) -> Result<Vec<RequestLogEntry>> {
        self.list_request_logs_filtered(limit, None, None)
    }

    pub fn list_request_logs_filtered(
        &self,
        limit: usize,
        operation: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<RequestLogEntry>> {
        let uri = request_log_uri()?;
        let raw = match self.fs.read(&uri) {
            Ok(r) => r,
            Err(crate::error::AxiomError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let operation = operation
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_ascii_lowercase);
        let status = status
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_ascii_lowercase);
        let parsed = parse_jsonl_tolerant::<RequestLogEntry>(&raw);
        if parsed.items.is_empty() && parsed.skipped_lines > 0 {
            return Err(jsonl_all_lines_invalid(
                "request log",
                None,
                parsed.skipped_lines,
                parsed.first_error.as_ref(),
            ));
        }

        let mut entries = parsed
            .items
            .into_iter()
            .filter(|e| {
                operation
                    .as_deref()
                    .is_none_or(|op| e.operation.eq_ignore_ascii_case(op))
            })
            .filter(|e| {
                status
                    .as_deref()
                    .is_none_or(|st| e.status.eq_ignore_ascii_case(st))
            })
            .collect::<Vec<_>>();
        entries.reverse();
        entries.truncate(limit.max(1));
        Ok(entries)
    }

    pub fn sessions(&self) -> Result<Vec<SessionInfo>> {
        self.session_service().sessions()
    }

    pub fn promote_session_memories(
        &self,
        request: &MemoryPromotionRequest,
    ) -> Result<MemoryPromotionResult> {
        self.session_service().promote_session_memories(request)
    }

    pub fn checkpoint_session_archive_only(&self, session_id: &str) -> Result<CommitResult> {
        self.session_service()
            .checkpoint_session_archive_only(session_id)
    }

    pub fn promote_and_checkpoint_archive_only(
        &self,
        request: &MemoryPromotionRequest,
    ) -> Result<MemoryPromotionResult> {
        self.session_service()
            .promote_and_checkpoint_archive_only(request)
    }

    pub fn delete(&self, session_id: &str) -> Result<bool> {
        self.session_service().delete(session_id)
    }

    pub fn reindex_all(&self) -> Result<()> {
        self.runtime_bootstrap_service().reindex_all()
    }

    fn session_updated_at(&self, session_uri: &AxiomUri) -> DateTime<Utc> {
        let session_path = self.fs.resolve_uri(session_uri);
        let meta_path = session_path.join(".meta.json");
        if let Ok(raw_meta) = fs::read_to_string(&meta_path)
            && let Ok(meta) = serde_json::from_str::<SessionMeta>(&raw_meta)
        {
            return meta.updated_at;
        }

        fs::metadata(&session_path)
            .and_then(|m| m.modified())
            .map_or_else(|_| Utc::now(), DateTime::<Utc>::from)
    }
}

fn saturating_duration_nanos_to_i64(duration: std::time::Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}

fn build_runtime_bootstrap_plan(
    current_stamp: String,
    stored_stamp: Option<String>,
    has_index_state_drift: bool,
    restored_search_documents: usize,
) -> RuntimeBootstrapPlan {
    let restore_decision =
        decide_runtime_restore(current_stamp, stored_stamp, has_index_state_drift, restored_search_documents);
    RuntimeBootstrapPlan {
        ensure_scope_tiers: true,
        restore_decision,
    }
}

fn decide_runtime_restore(
    current_stamp: String,
    stored_stamp: Option<String>,
    has_index_state_drift: bool,
    restored_search_documents: usize,
) -> RuntimeRestoreDecision {
    if stored_stamp.as_deref() != Some(current_stamp.as_str()) {
        return RuntimeRestoreDecision::FullReindex {
            reason: "index_profile_stamp_changed",
        };
    }
    if has_index_state_drift {
        return RuntimeRestoreDecision::FullReindex {
            reason: "index_state_drift",
        };
    }
    if restored_search_documents == 0 {
        return RuntimeRestoreDecision::FullReindex {
            reason: "state_restore_empty",
        };
    }
    RuntimeRestoreDecision::RestoreFromState {
        stamp: current_stamp,
    }
}

fn build_full_reindex_repair_plan(
    index_profile_stamp: String,
    repair_type: &'static str,
) -> IndexRepairPlan {
    IndexRepairPlan {
        repair_type,
        restore_source: "full_reindex",
        index_profile_stamp,
    }
}

fn build_session_scope_plan(session_id: &str) -> SessionScopePlan {
    SessionScopePlan {
        session_id: session_id.to_string(),
    }
}

fn build_promotion_execution_plan(
    session_id: String,
    checkpoint_after_promotion: bool,
) -> PromotionExecutionPlan {
    PromotionExecutionPlan {
        session_scope: SessionScopePlan { session_id },
        checkpoint_after_promotion,
    }
}

fn build_session_deletion_plan(
    session_id: &str,
    session_uri: AxiomUri,
    session_exists: bool,
) -> SessionDeletionPlan {
    SessionDeletionPlan {
        session_scope: build_session_scope_plan(session_id),
        session_uri,
        should_delete: session_exists,
    }
}

fn is_om_event_type(rate: &crate::models::QueueDeadLetterRate) -> bool {
    rate.event_type.starts_with("om_")
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeRestoreDecision, build_full_reindex_repair_plan,
        build_promotion_execution_plan, build_runtime_bootstrap_plan,
        build_session_deletion_plan, build_session_scope_plan, decide_runtime_restore,
    };
    use crate::{AxiomUri, Scope};

    #[test]
    fn runtime_restore_decision_reindexes_when_stamp_changes() {
        let decision = decide_runtime_restore("stamp-v2".to_string(), Some("stamp-v1".to_string()), false, 3);

        assert_eq!(
            decision,
            RuntimeRestoreDecision::FullReindex {
                reason: "index_profile_stamp_changed"
            }
        );
    }

    #[test]
    fn runtime_restore_decision_restores_when_state_is_current_and_non_empty() {
        let decision = decide_runtime_restore("stamp-v1".to_string(), Some("stamp-v1".to_string()), false, 2);

        assert_eq!(
            decision,
            RuntimeRestoreDecision::RestoreFromState {
                stamp: "stamp-v1".to_string()
            }
        );
    }

    #[test]
    fn bootstrap_plan_keeps_scope_tier_creation_and_restore_decision_together() {
        let plan = build_runtime_bootstrap_plan(
            "stamp-v1".to_string(),
            Some("stamp-v1".to_string()),
            true,
            4,
        );

        assert!(plan.ensure_scope_tiers);
        assert_eq!(
            plan.restore_decision,
            RuntimeRestoreDecision::FullReindex {
                reason: "index_state_drift"
            }
        );
    }

    #[test]
    fn full_reindex_repair_plan_records_restore_source_and_stamp() {
        let plan = build_full_reindex_repair_plan("stamp-v1".to_string(), "full_reindex");

        assert_eq!(plan.repair_type, "full_reindex");
        assert_eq!(plan.restore_source, "full_reindex");
        assert_eq!(plan.index_profile_stamp, "stamp-v1");
    }

    #[test]
    fn session_scope_plan_preserves_session_id() {
        let plan = build_session_scope_plan("s-1");

        assert_eq!(plan.session_id, "s-1");
    }

    #[test]
    fn promotion_execution_plan_marks_archive_checkpoint_follow_up() {
        let plan = build_promotion_execution_plan("s-2".to_string(), true);

        assert_eq!(plan.session_scope.session_id, "s-2");
        assert!(plan.checkpoint_after_promotion);
    }

    #[test]
    fn session_deletion_plan_skips_missing_sessions() {
        let uri = AxiomUri::root(Scope::Session).join("s-missing").expect("uri");
        let plan = build_session_deletion_plan("s-missing", uri.clone(), false);

        assert_eq!(plan.session_scope.session_id, "s-missing");
        assert_eq!(plan.session_uri, uri);
        assert!(!plan.should_delete);
    }
}
