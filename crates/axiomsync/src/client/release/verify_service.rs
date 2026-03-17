use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::error::Result;
use crate::models::{
    BackendStatus, MigrationApplyReport, MigrationInspectReport, MigrationRunRecord,
    RUN_STATUS_SUCCESS, ReleaseVerifyReport, RetrievalDoctorReport, StorageDoctorReport,
};
use crate::state::schema::{
    CONTEXT_SCHEMA_VERSION_KEY, INDEX_PROFILE_STAMP_KEY, RELEASE_CONTRACT_VERSION_KEY,
    RUNTIME_RESTORE_SOURCE_KEY, SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY,
};

use super::AxiomSync;

#[derive(Debug, Clone)]
struct RetrievalDoctorSnapshot {
    retrieval_backend: String,
    retrieval_backend_policy: String,
    local_records: usize,
    indexed_documents: usize,
    trace_count: usize,
    restore_source: Option<String>,
    fts_ready: bool,
}

#[derive(Debug, Clone)]
struct MigrationApplyPlan {
    backup_path: Option<String>,
    applied_run: MigrationRunRecord,
}

pub(crate) struct ReleaseVerificationService<'a> {
    app: &'a AxiomSync,
}

impl<'a> ReleaseVerificationService<'a> {
    pub(crate) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(super) fn doctor_storage(&self) -> Result<StorageDoctorReport> {
        Ok(StorageDoctorReport {
            context_schema_version: self.state_value(CONTEXT_SCHEMA_VERSION_KEY)?,
            search_docs_fts_schema_version: self.state_value(SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY)?,
            index_profile_stamp: self.state_value(INDEX_PROFILE_STAMP_KEY)?,
            release_contract_version: self.state_value(RELEASE_CONTRACT_VERSION_KEY)?,
            search_document_count: self.app.state.search_document_count()?,
            event_count: self.app.state.event_count()?,
            link_count: self.app.state.link_count()?,
            latest_migration_runs: self.app.state.list_migration_runs(5)?,
            latest_repair_runs: self.app.state.list_repair_runs(5)?,
        })
    }

    pub(super) fn doctor_retrieval(&self) -> Result<RetrievalDoctorReport> {
        let backend = self.app.backend_status()?;
        let snapshot = build_retrieval_doctor_snapshot(
            backend,
            self.app.state.search_document_count()?,
            self.app.state.trace_count()?,
            self.state_value(RUNTIME_RESTORE_SOURCE_KEY)?,
            self.app.state.fts_ready()?,
        );
        Ok(build_retrieval_doctor_report(snapshot))
    }

    pub(super) fn migrate_inspect(&self) -> Result<MigrationInspectReport> {
        let context_schema_version = self.state_value(CONTEXT_SCHEMA_VERSION_KEY)?;
        let search_docs_fts_schema_version =
            self.state_value(SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY)?;
        let release_contract_version = self.state_value(RELEASE_CONTRACT_VERSION_KEY)?;
        let pending_actions = build_pending_migration_actions(
            context_schema_version.as_deref(),
            search_docs_fts_schema_version.as_deref(),
            release_contract_version.as_deref(),
        );
        Ok(MigrationInspectReport {
            context_schema_version,
            search_docs_fts_schema_version,
            release_contract_version,
            latest_migration_runs: self.app.state.list_migration_runs(5)?,
            latest_repair_runs: self.app.state.list_repair_runs(5)?,
            pending_actions,
        })
    }

    pub(super) fn migrate_apply(&self, backup_dir: Option<&Path>) -> Result<MigrationApplyReport> {
        let inspect_before = self.migrate_inspect()?;
        let backup_path = backup_dir
            .map(|dir| -> Result<String> {
                fs::create_dir_all(dir)?;
                let target = dir.join(format!("context-{}.db", uuid::Uuid::new_v4().simple()));
                fs::copy(self.app.fs.root().join("context.db"), &target)?;
                Ok(target.to_string_lossy().to_string())
            })
            .transpose()?;

        let started_at = Utc::now().to_rfc3339();
        self.app.state.ensure_schema()?;
        let finished_at = Utc::now().to_rfc3339();
        let plan = build_migration_apply_plan(backup_path, started_at, finished_at);
        self.app.state.record_migration_run(&plan.applied_run)?;
        let inspect_after = self.migrate_inspect()?;

        Ok(MigrationApplyReport {
            backup_path: plan.backup_path,
            inspect_before,
            inspect_after,
            applied_run: plan.applied_run,
        })
    }

    pub(super) fn release_verify(&self) -> Result<ReleaseVerifyReport> {
        let storage = self.doctor_storage()?;
        let retrieval = self.doctor_retrieval()?;
        Ok(build_release_verify_report(
            Utc::now().to_rfc3339(),
            storage,
            retrieval,
        ))
    }

    fn state_value(&self, key: &str) -> Result<Option<String>> {
        self.app.state.get_system_value(key)
    }
}

fn build_retrieval_doctor_snapshot(
    backend: BackendStatus,
    indexed_documents: usize,
    trace_count: usize,
    restore_source: Option<String>,
    fts_ready: bool,
) -> RetrievalDoctorSnapshot {
    RetrievalDoctorSnapshot {
        retrieval_backend: backend.retrieval_backend,
        retrieval_backend_policy: backend.retrieval_backend_policy,
        local_records: backend.local_records,
        indexed_documents,
        trace_count,
        restore_source,
        fts_ready,
    }
}

fn build_retrieval_doctor_report(snapshot: RetrievalDoctorSnapshot) -> RetrievalDoctorReport {
    RetrievalDoctorReport {
        retrieval_backend: snapshot.retrieval_backend,
        retrieval_backend_policy: snapshot.retrieval_backend_policy,
        local_records: snapshot.local_records,
        indexed_documents: snapshot.indexed_documents,
        trace_count: snapshot.trace_count,
        restore_source: snapshot.restore_source,
        fts_ready: snapshot.fts_ready,
    }
}

fn build_pending_migration_actions(
    context_schema_version: Option<&str>,
    search_docs_fts_schema_version: Option<&str>,
    release_contract_version: Option<&str>,
) -> Vec<String> {
    [
        context_schema_version
            .is_none()
            .then_some("context_schema_version_missing"),
        search_docs_fts_schema_version
            .is_none()
            .then_some("search_docs_fts_schema_version_missing"),
        release_contract_version
            .is_none()
            .then_some("release_contract_version_missing"),
    ]
    .into_iter()
    .flatten()
    .map(str::to_string)
    .collect()
}

fn build_migration_apply_plan(
    backup_path: Option<String>,
    started_at: String,
    finished_at: String,
) -> MigrationApplyPlan {
    let details = Some(serde_json::json!({ "backup_path": &backup_path }));
    MigrationApplyPlan {
        backup_path,
        applied_run: MigrationRunRecord {
            run_id: format!("migration-{}", uuid::Uuid::new_v4().simple()),
            operation: "ensure_schema".to_string(),
            started_at,
            finished_at: Some(finished_at),
            status: RUN_STATUS_SUCCESS.to_string(),
            details,
        },
    }
}

fn build_release_verify_report(
    verified_at: String,
    storage: StorageDoctorReport,
    retrieval: RetrievalDoctorReport,
) -> ReleaseVerifyReport {
    let report = ReleaseVerifyReport {
        verified_at,
        healthy: false,
        storage,
        retrieval,
    };
    let healthy = report.is_healthy();
    ReleaseVerifyReport { healthy, ..report }
}

impl AxiomSync {
    pub fn doctor_storage(&self) -> Result<StorageDoctorReport> {
        self.release_verification_service().doctor_storage()
    }

    pub fn doctor_retrieval(&self) -> Result<RetrievalDoctorReport> {
        self.release_verification_service().doctor_retrieval()
    }

    pub fn migrate_inspect(&self) -> Result<MigrationInspectReport> {
        self.release_verification_service().migrate_inspect()
    }

    pub fn migrate_apply(&self, backup_dir: Option<&Path>) -> Result<MigrationApplyReport> {
        self.release_verification_service()
            .migrate_apply(backup_dir)
    }

    pub fn release_verify(&self) -> Result<ReleaseVerifyReport> {
        self.release_verification_service().release_verify()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_migration_apply_plan, build_pending_migration_actions, build_release_verify_report,
        build_retrieval_doctor_report, build_retrieval_doctor_snapshot,
    };
    use crate::models::{BackendStatus, EmbeddingBackendStatus, RetrievalDoctorReport};

    #[test]
    fn pending_migration_actions_only_include_missing_versions() {
        let pending = build_pending_migration_actions(Some("1"), None, Some("2026-03-01"));

        assert_eq!(
            pending,
            vec!["search_docs_fts_schema_version_missing".to_string()]
        );
    }

    #[test]
    fn migration_apply_plan_carries_backup_path_into_audit_details() {
        let plan = build_migration_apply_plan(
            Some("/tmp/context.db".to_string()),
            "2026-03-16T00:00:00Z".to_string(),
            "2026-03-16T00:00:01Z".to_string(),
        );

        assert_eq!(plan.applied_run.operation, "ensure_schema");
        assert_eq!(plan.applied_run.status, "success");
        assert_eq!(
            plan.applied_run.details.expect("details")["backup_path"],
            "/tmp/context.db"
        );
    }

    #[test]
    fn retrieval_doctor_snapshot_preserves_backend_contract_fields() {
        let snapshot = build_retrieval_doctor_snapshot(
            BackendStatus {
                local_records: 12,
                retrieval_backend: "memory".to_string(),
                retrieval_backend_policy: "memory_only".to_string(),
                embedding: EmbeddingBackendStatus {
                    provider: "mock".to_string(),
                    vector_version: "v1".to_string(),
                    dim: 384,
                },
            },
            7,
            3,
            Some("state_restore".to_string()),
            true,
        );

        let report: RetrievalDoctorReport = build_retrieval_doctor_report(snapshot);
        assert_eq!(report.retrieval_backend, "memory");
        assert_eq!(report.retrieval_backend_policy, "memory_only");
        assert_eq!(report.local_records, 12);
        assert_eq!(report.indexed_documents, 7);
        assert_eq!(report.trace_count, 3);
        assert_eq!(report.restore_source.as_deref(), Some("state_restore"));
        assert!(report.fts_ready);
    }

    #[test]
    fn release_verify_report_builder_assembles_storage_and_retrieval() {
        let storage = crate::models::StorageDoctorReport {
            context_schema_version: Some("1".to_string()),
            search_docs_fts_schema_version: Some("1".to_string()),
            index_profile_stamp: Some("stamp".to_string()),
            release_contract_version: Some("2026-03".to_string()),
            search_document_count: 5,
            event_count: 2,
            link_count: 1,
            latest_migration_runs: Vec::new(),
            latest_repair_runs: Vec::new(),
        };
        let retrieval = crate::models::RetrievalDoctorReport {
            retrieval_backend: "memory".to_string(),
            retrieval_backend_policy: "memory_only".to_string(),
            local_records: 5,
            indexed_documents: 5,
            trace_count: 2,
            restore_source: Some("full_reindex".to_string()),
            fts_ready: true,
        };

        let report =
            build_release_verify_report("2026-03-16T00:00:00Z".to_string(), storage, retrieval);
        assert_eq!(report.verified_at, "2026-03-16T00:00:00Z");
        assert_eq!(report.storage.search_document_count, 5);
        assert_eq!(
            report.retrieval.restore_source.as_deref(),
            Some("full_reindex")
        );
    }
}
