use std::io::Read;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::context_ops::default_resource_target;
use crate::error::{AxiomError, Result};
use crate::ingest::{IngestFinalizeMode, IngestManager, IngestSession};
use crate::models::{
    AddResourceIngestOptions, AddResourceRequest, AddResourceResult, AddResourceWaitMode,
    GlobResult, QueueCounts, QueueEventStatus, QueueStatus,
};
use crate::pack;
use crate::tier_documents::{read_abstract, read_overview};
use crate::uri::{AxiomUri, Scope};

use super::AxiomSync;

const MAX_REMOTE_TEXT_BYTES: usize = 5 * 1024 * 1024;
const WAIT_PROCESSED_MIN_SLEEP: Duration = Duration::from_millis(100);
const WAIT_PROCESSED_MAX_SLEEP: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddResourceIntent {
    source: String,
    target_uri: AxiomUri,
    wait: bool,
    timeout_secs: Option<u64>,
    wait_mode: AddResourceWaitMode,
    ingest_options: AddResourceIngestOptions,
    finalize_mode: IngestFinalizeMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WaitStrategyDecision {
    None,
    RelaxedReplay,
    StrictTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AddResourcePlan {
    source: String,
    target_uri: AxiomUri,
    timeout_secs: Option<u64>,
    ingest_options: AddResourceIngestOptions,
    finalize_mode: IngestFinalizeMode,
    wait_strategy: WaitStrategyDecision,
    result_message: String,
    result_wait_mode: Option<AddResourceWaitMode>,
    result_wait_contract: Option<String>,
}

pub(crate) struct ResourceService<'a> {
    app: &'a AxiomSync,
}

impl<'a> ResourceService<'a> {
    pub(crate) fn new(app: &'a AxiomSync) -> Self {
        Self { app }
    }

    pub(crate) fn add_resource(
        &self,
        path_or_url: &str,
        target: Option<&str>,
        wait: bool,
        timeout_secs: Option<u64>,
    ) -> Result<AddResourceResult> {
        let mut request = AddResourceRequest::new(path_or_url.to_string());
        request.target = target.map(ToString::to_string);
        request.wait = wait;
        request.timeout_secs = timeout_secs;
        request.ingest_options = AddResourceIngestOptions::default();
        self.add_resource_with_ingest_options(request)
    }

    pub(crate) fn add_resource_with_ingest_options(
        &self,
        request: AddResourceRequest,
    ) -> Result<AddResourceResult> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let started = Instant::now();
        let target_raw = request.target.clone();
        let source = request.source.clone();
        let wait = request.wait;
        let wait_mode = request.wait_mode;
        let ingest_options = request.ingest_options.clone();
        let output = (|| -> Result<AddResourceResult> {
            let plan = build_add_resource_plan(build_add_resource_intent(request)?);
            self.execute_add_resource_plan(&plan)
        })();
        let ingest_options_json = serde_json::to_value(&ingest_options).unwrap_or_else(|_| {
            serde_json::json!({
                "markdown_only": ingest_options.markdown_only,
                "include_hidden": ingest_options.include_hidden,
                "exclude_globs": ingest_options.exclude_globs,
            })
        });

        match output {
            Ok(result) => {
                self.app.log_request_status(
                    request_id,
                    "add_resource",
                    "ok",
                    started,
                    Some(result.root_uri.clone()),
                    Some(serde_json::json!({
                        "source": source,
                        "wait": wait,
                        "wait_mode": wait_mode,
                        "queued": result.queued,
                        "wait_contract": result.wait_contract,
                        "ingest_options": ingest_options_json,
                    })),
                );
                Ok(result)
            }
            Err(err) => {
                self.app.log_request_error(
                    request_id,
                    "add_resource",
                    started,
                    target_raw,
                    &err,
                    Some(serde_json::json!({
                        "source": source,
                        "wait": wait,
                        "wait_mode": wait_mode,
                        "ingest_options": ingest_options_json,
                    })),
                );
                Err(err)
            }
        }
    }

    fn execute_add_resource_plan(&self, plan: &AddResourcePlan) -> Result<AddResourceResult> {
        let ingest_manager =
            IngestManager::new(self.app.fs.clone(), self.app.parser_registry.clone());
        let mut ingest = ingest_manager.start_session()?;
        if let Err(err) = stage_add_resource_source(
            &plan.source,
            plan.timeout_secs,
            &mut ingest,
            &plan.ingest_options,
        ) {
            ingest.abort();
            return Err(err);
        }
        if let Err(err) = ingest.write_manifest(&plan.source) {
            ingest.abort();
            return Err(err);
        }
        if let Err(err) = ingest.finalize_to(&plan.target_uri, plan.finalize_mode) {
            ingest.abort();
            return Err(err);
        }
        let outbox_event_id = self.app.state.enqueue(
            "semantic_scan",
            &plan.target_uri.to_string(),
            serde_json::json!({"op": "add_resource"}),
        )?;
        match plan.wait_strategy {
            WaitStrategyDecision::None => {}
            WaitStrategyDecision::RelaxedReplay => {
                let _ = self.app.replay_outbox(256, false)?;
            }
            WaitStrategyDecision::StrictTerminal => {
                self.app
                    .wait_for_outbox_event_done_strict(outbox_event_id, plan.timeout_secs)?;
            }
        }
        Ok(build_add_resource_result(plan))
    }
}

impl AxiomSync {
    pub fn add_resource(
        &self,
        path_or_url: &str,
        target: Option<&str>,
        _reason: Option<&str>,
        _instruction: Option<&str>,
        wait: bool,
        timeout_secs: Option<u64>,
    ) -> Result<AddResourceResult> {
        self.resource_service()
            .add_resource(path_or_url, target, wait, timeout_secs)
    }

    pub fn add_resource_with_ingest_options(
        &self,
        request: AddResourceRequest,
    ) -> Result<AddResourceResult> {
        self.resource_service()
            .add_resource_with_ingest_options(request)
    }

    pub fn wait_processed(&self, timeout_secs: Option<u64>) -> Result<QueueStatus> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(30).max(1));
        let started = Instant::now();

        loop {
            let counts = self.state.queue_counts()?;
            if counts.new_total == 0 && counts.processing == 0 {
                return self.state.queue_status();
            }

            if counts.new_due > 0 {
                let replay_limit = counts.new_due.clamp(1, 256) as usize;
                let _ = self.replay_outbox(replay_limit, false)?;
            }

            let after = self.state.queue_counts()?;
            if after.new_total == 0 && after.processing == 0 {
                return self.state.queue_status();
            }

            if started.elapsed() >= timeout {
                return Err(AxiomError::Conflict(format!(
                    "wait_processed timeout after {}s: new_total={} processing={} dead_letter={}",
                    timeout.as_secs(),
                    after.new_total,
                    after.processing,
                    after.dead_letter
                )));
            }

            let timeout_remaining = timeout.saturating_sub(started.elapsed());
            let sleep_for = wait_processed_sleep_duration(&after, timeout_remaining);
            if !sleep_for.is_zero() {
                thread::sleep(sleep_for);
            }
        }
    }

    fn wait_for_outbox_event_done_strict(
        &self,
        outbox_event_id: i64,
        timeout_secs: Option<u64>,
    ) -> Result<()> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(30).max(1));
        let started = Instant::now();

        loop {
            let event = self
                .state
                .get_outbox_event(outbox_event_id)?
                .ok_or_else(|| {
                    AxiomError::Conflict(format!(
                        "strict wait failed: outbox event {outbox_event_id} not found"
                    ))
                })?;

            match event.status {
                QueueEventStatus::Done => return Ok(()),
                QueueEventStatus::DeadLetter => {
                    return Err(AxiomError::Conflict(format!(
                        "strict wait failed: outbox event {outbox_event_id} dead-lettered (attempt_count={})",
                        event.attempt_count
                    )));
                }
                QueueEventStatus::New | QueueEventStatus::Processing => {}
            }

            if started.elapsed() >= timeout {
                let counts = self.state.queue_counts()?;
                return Err(AxiomError::Conflict(format!(
                    "strict wait timeout after {}s: outbox event {} status={} pending_new={} processing={} dead_letter={} (pending/requeued/dead-letter remains)",
                    timeout.as_secs(),
                    outbox_event_id,
                    event.status,
                    counts.new_total,
                    counts.processing,
                    counts.dead_letter
                )));
            }

            let _ = self.replay_outbox(256, false)?;

            let counts = self.state.queue_counts()?;
            let timeout_remaining = timeout.saturating_sub(started.elapsed());
            let sleep_for = wait_processed_sleep_duration(&counts, timeout_remaining);
            if !sleep_for.is_zero() {
                thread::sleep(sleep_for);
            }
        }
    }

    pub fn ls(
        &self,
        uri: &str,
        recursive: bool,
        _simple: bool,
    ) -> Result<Vec<crate::models::Entry>> {
        let uri = AxiomUri::parse(uri)?;
        self.fs.list(&uri, recursive)
    }

    pub fn glob(&self, pattern: &str, uri: Option<&str>) -> Result<GlobResult> {
        let base = if let Some(raw) = uri {
            Some(AxiomUri::parse(raw)?)
        } else {
            None
        };
        let matches = self.fs.glob(base.as_ref(), pattern)?;
        Ok(GlobResult { matches })
    }

    pub fn read(&self, uri: &str) -> Result<String> {
        let uri = AxiomUri::parse(uri)?;
        self.fs.read(&uri)
    }

    pub fn abstract_text(&self, uri: &str) -> Result<String> {
        let uri = AxiomUri::parse(uri)?;
        read_abstract(&self.fs, &uri)
    }

    pub fn overview(&self, uri: &str) -> Result<String> {
        let uri = AxiomUri::parse(uri)?;
        read_overview(&self.fs, &uri)
    }

    pub fn mkdir(&self, uri: &str) -> Result<()> {
        let uri = AxiomUri::parse(uri)?;
        if !matches!(
            uri.scope(),
            Scope::Resources | Scope::User | Scope::Agent | Scope::Session
        ) {
            return Err(AxiomError::PermissionDenied(format!(
                "mkdir is not allowed for scope: {}",
                uri.scope()
            )));
        }

        self.fs.create_dir_all(&uri, false)?;
        self.reindex_uri_tree(&uri)?;
        self.state.enqueue(
            "reindex",
            &uri.to_string(),
            serde_json::json!({"op": "mkdir"}),
        )?;
        Ok(())
    }

    pub fn rm(&self, uri: &str, recursive: bool) -> Result<()> {
        let uri = AxiomUri::parse(uri)?;
        self.fs.rm(&uri, recursive, false)?;
        self.purge_uri_index(&uri)?;

        self.state.enqueue(
            "delete",
            &uri.to_string(),
            serde_json::json!({"op": "rm", "recursive": recursive}),
        )?;
        Ok(())
    }

    pub fn mv(&self, from_uri: &str, to_uri: &str) -> Result<()> {
        let from = AxiomUri::parse(from_uri)?;
        let to = AxiomUri::parse(to_uri)?;
        if from.scope() != to.scope() {
            return Err(AxiomError::PermissionDenied(format!(
                "cross-scope move is not allowed: {} -> {}",
                from.scope(),
                to.scope()
            )));
        }
        self.fs.mv(&from, &to, false)?;
        self.purge_uri_index(&from)?;
        self.reindex_uri_tree(&to)?;

        self.state.enqueue(
            "reindex",
            &to.to_string(),
            serde_json::json!({"op": "mv", "from": from_uri}),
        )?;
        Ok(())
    }

    pub fn tree(&self, uri: &str) -> Result<crate::models::TreeResult> {
        let uri = AxiomUri::parse(uri)?;
        self.fs.tree(&uri)
    }

    pub fn export_ovpack(&self, uri: &str, to: &str) -> Result<String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let started = Instant::now();
        let uri_raw = uri.to_string();
        let to_path = to.to_string();

        let output = (|| -> Result<String> {
            let uri = AxiomUri::parse(uri)?;
            if !matches!(
                uri.scope(),
                Scope::Resources | Scope::User | Scope::Agent | Scope::Session
            ) {
                return Err(AxiomError::PermissionDenied(
                    "ovpack export is not allowed for internal scopes".to_string(),
                ));
            }
            let out = pack::export_ovpack(&self.fs, &uri, Path::new(to))?;
            Ok(out.display().to_string())
        })();

        match output {
            Ok(export_path) => {
                self.log_request_status(
                    request_id,
                    "ovpack.export",
                    "ok",
                    started,
                    Some(uri_raw),
                    Some(serde_json::json!({
                        "to": to_path,
                        "output": export_path,
                    })),
                );
                Ok(export_path)
            }
            Err(err) => {
                self.log_request_error(
                    request_id,
                    "ovpack.export",
                    started,
                    Some(uri_raw),
                    &err,
                    Some(serde_json::json!({
                        "to": to_path,
                    })),
                );
                Err(err)
            }
        }
    }

    pub fn import_ovpack(
        &self,
        file_path: &str,
        parent: &str,
        force: bool,
        vectorize: bool,
    ) -> Result<String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let started = Instant::now();
        let file_path_raw = file_path.to_string();
        let parent_raw = parent.to_string();

        let output = (|| -> Result<String> {
            let parent_uri = AxiomUri::parse(parent)?;
            if !matches!(
                parent_uri.scope(),
                Scope::Resources | Scope::User | Scope::Agent | Scope::Session
            ) {
                return Err(AxiomError::PermissionDenied(
                    "ovpack import is not allowed for internal scopes".to_string(),
                ));
            }
            let imported = pack::import_ovpack(&self.fs, Path::new(file_path), &parent_uri, force)?;
            if vectorize {
                self.purge_uri_index(&imported)?;
                self.ensure_tiers_recursive(&imported)?;
                self.reindex_uri_tree(&imported)?;
            }
            Ok(imported.to_string())
        })();

        match output {
            Ok(imported_uri) => {
                self.log_request_status(
                    request_id,
                    "ovpack.import",
                    "ok",
                    started,
                    Some(parent_raw),
                    Some(serde_json::json!({
                        "file_path": file_path_raw,
                        "force": force,
                        "vectorize": vectorize,
                        "imported_uri": imported_uri,
                    })),
                );
                Ok(imported_uri)
            }
            Err(err) => {
                self.log_request_error(
                    request_id,
                    "ovpack.import",
                    started,
                    Some(parent_raw),
                    &err,
                    Some(serde_json::json!({
                        "file_path": file_path_raw,
                        "force": force,
                        "vectorize": vectorize,
                    })),
                );
                Err(err)
            }
        }
    }
}

impl AxiomSync {
    pub(super) fn purge_uri_index(&self, uri: &AxiomUri) -> Result<()> {
        self.prune_index_prefix_from_memory(uri)?;
        self.state.purge_uri_prefix_state(&uri.to_string())?;
        Ok(())
    }

    pub(super) fn prune_index_prefix_from_memory(&self, prefix: &AxiomUri) -> Result<Vec<String>> {
        let doomed = {
            let mut index = self
                .index
                .write()
                .map_err(|_| AxiomError::lock_poisoned("index"))?;
            let doomed = index.uris_with_prefix(prefix);
            for uri in &doomed {
                index.remove(uri);
            }
            doomed
        };
        Ok(doomed)
    }
}

fn build_add_resource_intent(request: AddResourceRequest) -> Result<AddResourceIntent> {
    let target_uri = request
        .target
        .as_deref()
        .map(AxiomUri::parse)
        .transpose()?
        .map_or_else(|| default_resource_target(&request.source), Ok)?;
    let finalize_mode = resolve_add_resource_finalize_mode(&request.source)?;
    Ok(AddResourceIntent {
        source: request.source,
        target_uri,
        wait: request.wait,
        timeout_secs: request.timeout_secs,
        wait_mode: request.wait_mode,
        ingest_options: request.ingest_options,
        finalize_mode,
    })
}

fn build_add_resource_plan(intent: AddResourceIntent) -> AddResourcePlan {
    let wait_strategy = match (intent.wait, intent.wait_mode) {
        (false, _) => WaitStrategyDecision::None,
        (true, AddResourceWaitMode::Relaxed) => WaitStrategyDecision::RelaxedReplay,
        (true, AddResourceWaitMode::Strict) => WaitStrategyDecision::StrictTerminal,
    };
    AddResourcePlan {
        source: intent.source,
        target_uri: intent.target_uri,
        timeout_secs: intent.timeout_secs,
        ingest_options: intent.ingest_options,
        finalize_mode: intent.finalize_mode,
        result_message: if intent.wait {
            "resource ingested".to_string()
        } else {
            "resource staged and queued for semantic processing".to_string()
        },
        result_wait_mode: intent.wait.then_some(intent.wait_mode),
        result_wait_contract: intent
            .wait
            .then_some(intent.wait_mode.contract_label().to_string()),
        wait_strategy,
    }
}

fn build_add_resource_result(plan: &AddResourcePlan) -> AddResourceResult {
    AddResourceResult {
        root_uri: plan.target_uri.to_string(),
        queued: matches!(plan.wait_strategy, WaitStrategyDecision::None),
        message: plan.result_message.clone(),
        wait_mode: plan.result_wait_mode,
        wait_contract: plan.result_wait_contract.clone(),
    }
}

fn resolve_add_resource_finalize_mode(path_or_url: &str) -> Result<IngestFinalizeMode> {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        return Ok(IngestFinalizeMode::MergeIntoTarget);
    }
    match std::fs::metadata(path_or_url) {
        Ok(meta) => Ok(if meta.is_dir() {
            IngestFinalizeMode::ReplaceTarget
        } else {
            IngestFinalizeMode::MergeIntoTarget
        }),
        Err(_) => Err(AxiomError::NotFound(path_or_url.to_string())),
    }
}

fn stage_add_resource_source(
    path_or_url: &str,
    timeout_secs: Option<u64>,
    ingest: &mut IngestSession,
    ingest_options: &AddResourceIngestOptions,
) -> Result<()> {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(30).max(1));
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()?;
        let resp = client.get(path_or_url).send()?;
        if !resp.status().is_success() {
            return Err(AxiomError::Internal(format!(
                "failed to fetch {path_or_url}: status {}",
                resp.status()
            )));
        }
        if let Some(bytes) = resp.content_length()
            && bytes > MAX_REMOTE_TEXT_BYTES as u64
        {
            return Err(AxiomError::Validation(format!(
                "remote resource too large: {bytes} bytes (limit {MAX_REMOTE_TEXT_BYTES})"
            )));
        }
        let text = read_remote_text_limited(resp, MAX_REMOTE_TEXT_BYTES)?;
        ingest.stage_text("source.txt", &text)?;
        return Ok(());
    }

    ingest.stage_local_path_with_options(Path::new(path_or_url), ingest_options)?;
    Ok(())
}

fn read_remote_text_limited<R: Read>(mut reader: R, max_bytes: usize) -> Result<String> {
    let mut body = Vec::new();
    let mut limited = (&mut reader).take((max_bytes as u64) + 1);
    limited.read_to_end(&mut body)?;
    if body.len() > max_bytes {
        return Err(AxiomError::Validation(format!(
            "remote resource too large after download: {} bytes (limit {max_bytes})",
            body.len()
        )));
    }
    String::from_utf8(body).map_err(|err| {
        AxiomError::Validation(format!("remote resource is not valid utf-8 text: {err}"))
    })
}

fn wait_processed_sleep_duration(counts: &QueueCounts, timeout_remaining: Duration) -> Duration {
    if timeout_remaining.is_zero() {
        return Duration::ZERO;
    }

    let fallback_sleep = WAIT_PROCESSED_MIN_SLEEP.min(timeout_remaining);
    if counts.new_due > 0 {
        return fallback_sleep;
    }

    let Some(raw_due_at) = counts.earliest_next_attempt_at.as_deref() else {
        return fallback_sleep;
    };
    let Ok(parsed_due_at) = DateTime::parse_from_rfc3339(raw_due_at) else {
        return fallback_sleep;
    };

    let due_at_utc = parsed_due_at.with_timezone(&Utc);
    let now = Utc::now();
    if due_at_utc <= now {
        return fallback_sleep;
    }

    let Ok(until_due) = due_at_utc.signed_duration_since(now).to_std() else {
        return fallback_sleep;
    };
    until_due
        .clamp(WAIT_PROCESSED_MIN_SLEEP, WAIT_PROCESSED_MAX_SLEEP)
        .min(timeout_remaining)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;

    use chrono::Duration as ChronoDuration;
    use tempfile::tempdir;

    use super::*;
    use crate::client::AxiomSync;
    use crate::models::{AddResourceIngestOptions, AddResourceWaitMode};
    use crate::uri::AxiomUri;

    #[test]
    fn read_remote_text_limited_rejects_payload_over_limit() {
        let data = vec![b'a'; MAX_REMOTE_TEXT_BYTES + 1];
        let err = read_remote_text_limited(Cursor::new(data), MAX_REMOTE_TEXT_BYTES)
            .expect_err("must reject oversized payload");
        assert!(matches!(err, AxiomError::Validation(_)));
    }

    #[test]
    fn read_remote_text_limited_accepts_payload_within_limit() {
        let data = b"hello remote".to_vec();
        let text =
            read_remote_text_limited(Cursor::new(data.clone()), MAX_REMOTE_TEXT_BYTES).expect("ok");
        assert_eq!(text, String::from_utf8(data).expect("utf8"));
    }

    #[test]
    fn add_resource_plan_uses_default_target_and_relaxed_wait_strategy() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("demo.md");
        fs::write(&source, "# demo").expect("write");
        let source = source.to_string_lossy().to_string();
        let plan = build_add_resource_plan(
            build_add_resource_intent(AddResourceRequest::new(source.clone())).expect("intent"),
        );

        assert_eq!(plan.source, source);
        assert_eq!(plan.wait_strategy, WaitStrategyDecision::None);
        assert!(build_add_resource_result(&plan).queued);
        assert_eq!(
            plan.target_uri.to_string(),
            default_resource_target(&source)
                .expect("default target")
                .to_string()
        );
    }

    #[test]
    fn add_resource_plan_records_strict_wait_contract() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("strict.md");
        fs::write(&source, "# strict").expect("write");

        let mut request = AddResourceRequest::new(source.to_str().expect("path"));
        request.target = Some("axiom://resources/strict-plan".to_string());
        request.wait = true;
        request.wait_mode = AddResourceWaitMode::Strict;
        let plan = build_add_resource_plan(build_add_resource_intent(request).expect("intent"));

        assert_eq!(plan.wait_strategy, WaitStrategyDecision::StrictTerminal);
        assert_eq!(plan.result_wait_mode, Some(AddResourceWaitMode::Strict));
        assert_eq!(
            plan.result_wait_contract.as_deref(),
            Some(AddResourceWaitMode::Strict.contract_label())
        );
    }

    #[test]
    fn wait_processed_drains_pending_queue_work() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app new");
        app.initialize().expect("init");

        let src = temp.path().join("wait_processed.txt");
        fs::write(&src, "OAuth wait processed flow").expect("write");
        app.add_resource(
            src.to_str().expect("src"),
            Some("axiom://resources/wait-processed"),
            None,
            None,
            false,
            None,
        )
        .expect("add");

        let status = app.wait_processed(Some(5)).expect("wait");
        assert!(status.semantic.processed >= 1);

        let counts = app.state.queue_counts().expect("queue counts");
        assert_eq!(counts.new_total, 0);
        assert_eq!(counts.processing, 0);
    }

    #[test]
    fn wait_processed_times_out_when_retries_are_backed_off() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app new");
        app.initialize().expect("init");

        app.state
            .enqueue("semantic_scan", "invalid://uri", serde_json::json!({}))
            .expect("enqueue");

        let err = app
            .wait_processed(Some(1))
            .expect_err("must timeout while event is delayed");
        assert!(matches!(err, AxiomError::Conflict(_)));
    }

    #[test]
    fn wait_processed_sleep_duration_uses_timeout_remaining_bound() {
        let counts = QueueCounts {
            new_total: 1,
            new_due: 0,
            processing: 0,
            done: 0,
            dead_letter: 0,
            earliest_next_attempt_at: Some((Utc::now() + ChronoDuration::seconds(30)).to_rfc3339()),
        };

        let sleep = wait_processed_sleep_duration(&counts, Duration::from_millis(250));
        assert_eq!(sleep, Duration::from_millis(250));
    }

    #[test]
    fn wait_processed_sleep_duration_falls_back_for_invalid_due_timestamp() {
        let counts = QueueCounts {
            new_total: 1,
            new_due: 0,
            processing: 0,
            done: 0,
            dead_letter: 0,
            earliest_next_attempt_at: Some("not-a-timestamp".to_string()),
        };

        let sleep = wait_processed_sleep_duration(&counts, Duration::from_secs(5));
        assert_eq!(sleep, WAIT_PROCESSED_MIN_SLEEP);
    }

    #[test]
    fn wait_processed_sleep_duration_prefers_min_when_due_work_exists() {
        let counts = QueueCounts {
            new_total: 2,
            new_due: 1,
            processing: 0,
            done: 0,
            dead_letter: 0,
            earliest_next_attempt_at: Some((Utc::now() + ChronoDuration::seconds(30)).to_rfc3339()),
        };

        let sleep = wait_processed_sleep_duration(&counts, Duration::from_secs(5));
        assert_eq!(sleep, WAIT_PROCESSED_MIN_SLEEP);
    }

    #[test]
    fn add_resource_with_markdown_only_options_filters_non_markdown_and_hidden_entries() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let source = temp.path().join("vault");
        fs::create_dir_all(source.join("nested")).expect("mkdir nested");
        fs::create_dir_all(source.join(".obsidian")).expect("mkdir hidden");
        fs::write(source.join("keep.md"), "# keep").expect("write keep");
        fs::write(source.join("nested").join("also.markdown"), "# keep nested")
            .expect("write nested keep");
        fs::write(source.join("drop.json"), "{\"drop\":true}").expect("write drop json");
        fs::write(source.join(".obsidian").join("drop.md"), "# drop hidden")
            .expect("write hidden drop");

        let mut request =
            AddResourceRequest::new(source.to_str().expect("source path").to_string());
        request.target = Some("axiom://resources/filtered".to_string());
        request.wait = true;
        request.ingest_options = AddResourceIngestOptions::markdown_only_defaults();
        app.add_resource_with_ingest_options(request)
            .expect("add filtered");

        let uris = app
            .state
            .list_search_documents()
            .expect("list")
            .into_iter()
            .map(|record| record.uri)
            .collect::<Vec<_>>();
        assert!(
            uris.iter()
                .any(|uri| uri == "axiom://resources/filtered/keep.md")
        );
        assert!(
            uris.iter()
                .any(|uri| uri == "axiom://resources/filtered/nested/also.markdown")
        );
        assert!(
            !uris
                .iter()
                .any(|uri| uri == "axiom://resources/filtered/drop.json")
        );
        assert!(
            !uris
                .iter()
                .any(|uri| uri == "axiom://resources/filtered/.obsidian/drop.md")
        );
    }

    #[test]
    fn add_resource_wait_relaxed_exposes_wait_contract_in_result() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let source = temp.path().join("relaxed.txt");
        fs::write(&source, "OAuth relaxed wait contract").expect("write source");

        let result = app
            .add_resource(
                source.to_str().expect("source path"),
                Some("axiom://resources/wait-relaxed"),
                None,
                None,
                true,
                None,
            )
            .expect("add relaxed");

        assert_eq!(result.wait_mode, Some(AddResourceWaitMode::Relaxed));
        assert_eq!(
            result.wait_contract.as_deref(),
            Some(AddResourceWaitMode::Relaxed.contract_label())
        );
    }

    #[test]
    fn add_resource_wait_strict_exposes_wait_contract_and_search_visibility() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let source = temp.path().join("strict.txt");
        fs::write(&source, "OAuth strict wait contract").expect("write source");

        let mut request = AddResourceRequest::new(source.to_str().expect("source path"));
        request.target = Some("axiom://resources/wait-strict".to_string());
        request.wait = true;
        request.wait_mode = AddResourceWaitMode::Strict;
        let result = app
            .add_resource_with_ingest_options(request)
            .expect("add strict");

        assert_eq!(result.wait_mode, Some(AddResourceWaitMode::Strict));
        assert_eq!(
            result.wait_contract.as_deref(),
            Some(AddResourceWaitMode::Strict.contract_label())
        );

        let hits = app
            .find(
                "oauth",
                Some("axiom://resources/wait-strict"),
                Some(5),
                None,
                None,
            )
            .expect("find strict");
        assert!(!hits.query_results.is_empty());
    }

    #[test]
    fn wait_for_outbox_event_done_strict_rejects_dead_letter_terminal_state() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let event_id = app
            .state
            .enqueue(
                "unknown_event_type",
                "axiom://resources/wait-strict-dead-letter",
                serde_json::json!({}),
            )
            .expect("enqueue");
        let err = app
            .wait_for_outbox_event_done_strict(event_id, Some(2))
            .expect_err("strict wait must fail");
        let message = format!("{err}");
        assert!(message.contains("dead-lettered"));

        let event = app
            .state
            .get_outbox_event(event_id)
            .expect("event lookup")
            .expect("event must exist");
        assert_eq!(event.status, QueueEventStatus::DeadLetter);
    }

    #[test]
    fn add_resource_file_keeps_existing_target_files() {
        let temp = tempdir().expect("tempdir");
        let app = AxiomSync::new(temp.path()).expect("app");
        app.initialize().expect("init");

        let source_dir = temp.path().join("source");
        fs::create_dir_all(&source_dir).expect("mkdir source");
        let first = source_dir.join("first.md");
        let second = source_dir.join("second.md");
        fs::write(&first, "# first").expect("write first");
        fs::write(&second, "# second").expect("write second");

        app.add_resource(
            first.to_str().expect("first path"),
            Some("axiom://resources/append"),
            None,
            None,
            false,
            None,
        )
        .expect("add first");
        app.add_resource(
            second.to_str().expect("second path"),
            Some("axiom://resources/append"),
            None,
            None,
            false,
            None,
        )
        .expect("add second");

        let first_uri = AxiomUri::parse("axiom://resources/append/first.md").expect("first uri");
        let second_uri = AxiomUri::parse("axiom://resources/append/second.md").expect("second uri");
        assert!(
            app.fs.exists(&first_uri),
            "first file must remain after second add"
        );
        assert!(app.fs.exists(&second_uri), "second file must be present");
    }
}
