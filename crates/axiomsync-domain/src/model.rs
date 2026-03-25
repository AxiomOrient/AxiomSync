use std::collections::BTreeMap;

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{AxiomError, Result};

pub const KERNEL_SCHEMA_VERSION: &str = "axiomsync-kernel-v2";
pub const RAW_EVENT_TAXONOMY: &[&str] = &[
    "message_captured",
    "selection_captured",
    "command_started",
    "command_finished",
    "artifact_emitted",
    "verification_recorded",
    "task_state_imported",
    "approval_requested",
    "approval_resolved",
    "note_recorded",
];

pub fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            let ordered: BTreeMap<_, _> = map.iter().collect();
            for (key, value) in ordered {
                out.insert(key.clone(), canonical_json(value));
            }
            Value::Object(out)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

pub fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonical_json(value)).expect("canonical JSON")
}

pub fn stable_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0x1f]);
    }
    hex::encode(hasher.finalize())
}

pub fn stable_id(prefix: &str, value: &impl Serialize) -> String {
    let serialized = serde_json::to_value(value).expect("serializable id value");
    let canonical = canonical_json_string(&serialized);
    format!(
        "{prefix}_{}",
        &stable_hash(&[prefix, canonical.as_str()])[..16]
    )
}

pub fn workspace_stable_id(workspace_root: &str) -> String {
    stable_id("ws", &workspace_root)
}

pub fn normalize_search_query(query: &str) -> Option<String> {
    let tokens = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

pub fn ts_ms_to_rfc3339(ts_ms: i64) -> Result<String> {
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|value| value.to_rfc3339())
        .ok_or_else(|| AxiomError::Validation(format!("invalid timestamp millis {ts_ms}")))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RawArtifactInput {
    pub artifact_kind: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

impl RawArtifactInput {
    pub fn validate(&self) -> Result<()> {
        if self.artifact_kind.trim().is_empty() || self.uri.trim().is_empty() {
            return Err(AxiomError::Validation(
                "raw artifact requires artifact_kind and uri".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawEventInput {
    #[serde(default, alias = "source")]
    pub connector: String,
    pub native_schema_version: Option<String>,
    #[serde(default)]
    pub session_kind: Option<String>,
    #[serde(default, alias = "native_session_id")]
    pub external_session_key: Option<String>,
    #[serde(default, alias = "native_event_id", alias = "native_entry_id")]
    pub external_entry_key: Option<String>,
    #[serde(default, alias = "event_type")]
    pub event_kind: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub captured_at: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub dedupe_key: Option<String>,
    #[serde(default)]
    pub ts_ms: Option<i64>,
    #[serde(default)]
    pub observed_at_ms: Option<i64>,
    #[serde(default)]
    pub captured_at_ms: Option<i64>,
    #[serde(default = "empty_object")]
    pub payload: Value,
    #[serde(default)]
    pub raw_payload: Option<Value>,
    #[serde(default)]
    pub artifacts: Vec<RawArtifactInput>,
    #[serde(default = "empty_object")]
    pub hints: Value,
}

impl RawEventInput {
    pub fn validate_event_type(&self) -> Result<()> {
        let event_type = self.normalized_event_kind()?;
        if RAW_EVENT_TAXONOMY.contains(&event_type.as_str()) {
            Ok(())
        } else {
            Err(AxiomError::Validation(format!(
                "unsupported raw event event_type `{event_type}`"
            )))
        }
    }

    pub fn normalized_session_kind(&self) -> &str {
        self.session_kind
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                self.hints
                    .get("session_kind")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                self.payload
                    .get("session_kind")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or("thread")
    }

    pub fn normalized_workspace_root(&self) -> Option<String> {
        self.workspace_root
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                self.hints
                    .get("workspace_root")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(ToOwned::to_owned)
            })
            .or_else(|| {
                self.payload
                    .get("workspace_root")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .map(ToOwned::to_owned)
            })
    }

    pub fn normalized_session_key(&self) -> Result<String> {
        self.external_session_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AxiomError::Validation(
                    "raw event input requires external_session_key or native_session_id"
                        .to_string(),
                )
            })
    }

    pub fn normalized_event_kind(&self) -> Result<String> {
        self.event_kind
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AxiomError::Validation("raw event input requires event_kind".to_string())
            })
    }

    pub fn normalized_observed_at(&self) -> Result<String> {
        if let Some(value) = self
            .observed_at
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(value.to_string());
        }
        if let Some(ts_ms) = self.observed_at_ms {
            return ts_ms_to_rfc3339(ts_ms);
        }
        if let Some(ts_ms) = self.ts_ms {
            return ts_ms_to_rfc3339(ts_ms);
        }
        Err(AxiomError::Validation(
            "raw event input requires observed_at, observed_at_ms, or ts_ms".to_string(),
        ))
    }

    pub fn normalized_captured_at(&self) -> Result<Option<String>> {
        if let Some(value) = self
            .captured_at
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(Some(value.to_string()));
        }
        if let Some(ts_ms) = self.captured_at_ms {
            return Ok(Some(ts_ms_to_rfc3339(ts_ms)?));
        }
        self.ts_ms.map(ts_ms_to_rfc3339).transpose()
    }

    pub fn normalized_content_hash(&self) -> Result<String> {
        if let Some(hash) = self
            .content_hash
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(hash.to_string());
        }
        let mut hasher = Sha256::new();
        hasher.update(canonical_json_string(&self.payload));
        if let Some(raw_payload) = &self.raw_payload {
            hasher.update(canonical_json_string(raw_payload));
        }
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn validate(&self) -> Result<()> {
        if self.connector.trim().is_empty() {
            return Err(AxiomError::Validation(
                "raw event input requires connector".to_string(),
            ));
        }
        self.normalized_session_key()?;
        self.validate_event_type()?;
        self.normalized_observed_at()?;
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppendRawEventsRequest {
    pub batch_id: String,
    pub producer: String,
    pub received_at_ms: i64,
    pub events: Vec<RawEventInput>,
}

impl AppendRawEventsRequest {
    pub fn validate(&self) -> Result<()> {
        if self.batch_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "append_raw_events requires batch_id".to_string(),
            ));
        }
        if self.producer.trim().is_empty() {
            return Err(AxiomError::Validation(
                "append_raw_events requires producer".to_string(),
            ));
        }
        if self.received_at_ms < 0 {
            return Err(AxiomError::Validation(
                "append_raw_events requires non-negative received_at_ms".to_string(),
            ));
        }
        if self.events.is_empty() {
            return Err(AxiomError::Validation(
                "append_raw_events requires at least one event".to_string(),
            ));
        }
        for event in &self.events {
            event.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpsertSourceCursorRequest {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

impl UpsertSourceCursorRequest {
    pub fn validate(&self) -> Result<()> {
        if self.connector.trim().is_empty()
            || self.cursor_key.trim().is_empty()
            || self.cursor_value.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "upsert_source_cursor requires connector, cursor_key, and cursor_value".to_string(),
            ));
        }
        if self.updated_at_ms < 0 {
            return Err(AxiomError::Validation(
                "upsert_source_cursor requires non-negative updated_at_ms".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRef {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_hex: Option<String>,
}

impl ArtifactRef {
    pub fn validate(&self, field_name: &str) -> Result<()> {
        if self.uri.trim().is_empty() {
            return Err(AxiomError::Validation(format!(
                "{field_name}.uri must not be empty"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandPayload {
    pub argv: Vec<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    #[serde(default)]
    pub env_keys: Vec<String>,
}

impl CommandPayload {
    pub fn validate(&self) -> Result<()> {
        if self.argv.is_empty() {
            return Err(AxiomError::Validation(
                "command.argv must not be empty".to_string(),
            ));
        }
        if self.cwd.trim().is_empty() {
            return Err(AxiomError::Validation(
                "command.cwd must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationPayload {
    pub kind: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl VerificationPayload {
    pub fn validate(&self) -> Result<()> {
        if self.kind.trim().is_empty() || self.status.trim().is_empty() {
            return Err(AxiomError::Validation(
                "verification.kind and verification.status must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatgptSelectionPayload {
    pub conversation_id: String,
    pub message_id: String,
    pub role: String,
    pub selected_text: String,
    pub start_hint: String,
    pub end_hint: String,
    pub dom_fingerprint: String,
    pub page_url: String,
    pub page_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_note: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ChatgptSelectionPayload {
    pub fn validate(&self) -> Result<()> {
        for (field, value) in [
            ("conversation_id", self.conversation_id.as_str()),
            ("message_id", self.message_id.as_str()),
            ("role", self.role.as_str()),
            ("selected_text", self.selected_text.as_str()),
            ("start_hint", self.start_hint.as_str()),
            ("end_hint", self.end_hint.as_str()),
            ("dom_fingerprint", self.dom_fingerprint.as_str()),
            ("page_url", self.page_url.as_str()),
            ("page_title", self.page_title.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AxiomError::Validation(format!(
                    "chatgpt selection field `{field}` must not be empty"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliCommandPayload {
    pub run_id: String,
    pub command_event_id: String,
    pub workspace_root: String,
    pub task_id: String,
    pub actor: String,
    pub command: CommandPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_artifact: Option<ArtifactRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_artifact: Option<ArtifactRef>,
    #[serde(default)]
    pub changed_files: Vec<String>,
    pub verification: VerificationPayload,
    pub finished_at_ms: u64,
}

impl CliCommandPayload {
    pub fn validate(&self) -> Result<()> {
        if self.run_id.trim().is_empty()
            || self.command_event_id.trim().is_empty()
            || self.workspace_root.trim().is_empty()
            || self.task_id.trim().is_empty()
            || self.actor.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "cli command payload requires run_id, command_event_id, workspace_root, task_id, and actor".to_string(),
            ));
        }
        self.command.validate()?;
        self.verification.validate()?;
        if let Some(artifact) = &self.stdout_artifact {
            artifact.validate("stdout_artifact")?;
        }
        if let Some(artifact) = &self.stderr_artifact {
            artifact.validate("stderr_artifact")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkStateExportPayload {
    pub snapshot_id: String,
    pub exported_at_ms: u64,
    pub workspace_root: String,
    pub run_id: String,
    pub task_id: String,
    pub status: String,
    pub progress_summary: String,
    pub task_file_uri: String,
    pub result_file_uri: String,
    pub events_file_uri: String,
    #[serde(default)]
    pub evidence_uris: Vec<String>,
}

impl WorkStateExportPayload {
    pub fn validate(&self) -> Result<()> {
        for (field, value) in [
            ("snapshot_id", self.snapshot_id.as_str()),
            ("workspace_root", self.workspace_root.as_str()),
            ("run_id", self.run_id.as_str()),
            ("task_id", self.task_id.as_str()),
            ("status", self.status.as_str()),
            ("progress_summary", self.progress_summary.as_str()),
            ("task_file_uri", self.task_file_uri.as_str()),
            ("result_file_uri", self.result_file_uri.as_str()),
            ("events_file_uri", self.events_file_uri.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AxiomError::Validation(format!(
                    "work state export field `{field}` must not be empty"
                )));
            }
        }
        if self.evidence_uris.iter().any(|uri| uri.trim().is_empty()) {
            return Err(AxiomError::Validation(
                "work state export evidence_uris must not contain empty values".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngressReceiptRow {
    pub receipt_id: String,
    pub batch_id: String,
    pub source_kind: String,
    pub connector: String,
    pub session_kind: String,
    pub external_session_key: Option<String>,
    pub external_entry_key: Option<String>,
    pub event_kind: String,
    pub observed_at: String,
    pub captured_at: Option<String>,
    pub workspace_root: Option<String>,
    pub content_hash: String,
    pub dedupe_key: Option<String>,
    pub payload_json: String,
    pub raw_payload_json: Option<String>,
    pub artifacts_json: String,
    pub normalized_json: String,
    pub projection_state: String,
    pub derived_state: String,
    pub index_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceCursorRow {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at: String,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestPlan {
    pub receipts: Vec<IngressReceiptRow>,
    pub cursor_update: Option<SourceCursorRow>,
    pub skipped_dedupe_keys: Vec<String>,
}

impl IngestPlan {
    pub fn validate(&self) -> Result<()> {
        for receipt in &self.receipts {
            if receipt.receipt_id.trim().is_empty()
                || receipt.batch_id.trim().is_empty()
                || receipt.source_kind.trim().is_empty()
                || receipt.connector.trim().is_empty()
                || receipt.session_kind.trim().is_empty()
                || receipt.event_kind.trim().is_empty()
                || receipt.observed_at.trim().is_empty()
                || receipt.content_hash.trim().is_empty()
            {
                return Err(AxiomError::Validation(
                    "ingest receipt is missing required fields".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceCursorUpsertPlan {
    pub cursor: SourceCursorRow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRow {
    pub session_id: String,
    pub session_kind: String,
    pub connector: String,
    pub external_session_key: Option<String>,
    pub title: Option<String>,
    pub workspace_root: Option<String>,
    pub opened_at: Option<String>,
    pub closed_at: Option<String>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActorRow {
    pub actor_id: String,
    pub actor_kind: String,
    pub stable_key: Option<String>,
    pub display_name: Option<String>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryRow {
    pub entry_id: String,
    pub session_id: String,
    pub seq_no: i64,
    pub entry_kind: String,
    pub actor_id: Option<String>,
    pub parent_entry_id: Option<String>,
    pub external_entry_key: Option<String>,
    pub text_body: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRow {
    pub artifact_id: String,
    pub session_id: String,
    pub entry_id: Option<String>,
    pub artifact_kind: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorRow {
    pub anchor_id: String,
    pub entry_id: Option<String>,
    pub artifact_id: Option<String>,
    pub anchor_kind: String,
    pub locator_json: String,
    pub preview_text: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionPlan {
    pub sessions: Vec<SessionRow>,
    pub actors: Vec<ActorRow>,
    pub entries: Vec<EntryRow>,
    pub artifacts: Vec<ArtifactRow>,
    pub anchors: Vec<AnchorRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeRow {
    pub episode_id: String,
    pub session_id: Option<String>,
    pub episode_kind: String,
    pub summary: String,
    pub status: Option<String>,
    pub confidence: f64,
    pub extractor_version: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightRow {
    pub insight_id: String,
    pub episode_id: Option<String>,
    pub insight_kind: String,
    pub statement: String,
    pub confidence: f64,
    #[serde(default = "empty_object")]
    pub scope_json: Value,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsightAnchorRow {
    pub insight_id: String,
    pub anchor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationRow {
    pub verification_id: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub method: String,
    pub status: String,
    pub checked_at: String,
    pub checker: Option<String>,
    #[serde(default = "empty_object")]
    pub details_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimRow {
    pub claim_id: String,
    pub episode_id: Option<String>,
    pub claim_kind: String,
    pub statement: String,
    pub confidence: f64,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimEvidenceRow {
    pub claim_id: String,
    pub anchor_id: String,
    pub support_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureRow {
    pub procedure_id: String,
    pub episode_id: Option<String>,
    pub title: String,
    pub goal: Option<String>,
    #[serde(default)]
    pub steps_json: Value,
    pub status: Option<String>,
    pub confidence: f64,
    pub extractor_version: String,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcedureEvidenceRow {
    pub procedure_id: String,
    pub anchor_id: String,
    pub support_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivePlan {
    pub episodes: Vec<EpisodeRow>,
    pub insights: Vec<InsightRow>,
    pub insight_anchors: Vec<InsightAnchorRow>,
    pub verifications: Vec<VerificationRow>,
    pub claims: Vec<ClaimRow>,
    pub claim_evidence: Vec<ClaimEvidenceRow>,
    pub procedures: Vec<ProcedureRow>,
    pub procedure_evidence: Vec<ProcedureEvidenceRow>,
    pub search_docs: Vec<SearchDocsRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayPlan {
    pub projection: ProjectionPlan,
    pub derivation: DerivePlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SearchFilter {
    pub session_kind: Option<String>,
    pub connector: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchCasesRequest {
    pub query: String,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub filter: SearchFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePreview {
    pub anchor_id: String,
    pub preview_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationSummary {
    pub status: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    #[serde(default)]
    pub evidence: Vec<EvidencePreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntryBundle {
    pub entry: EntryRow,
    pub artifacts: Vec<ArtifactRow>,
    pub anchors: Vec<AnchorRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionView {
    pub session: SessionRow,
    pub entries: Vec<EntryBundle>,
}

pub type ThreadView = SessionView;
pub type RunView = SessionView;
pub type TaskView = SessionView;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactView {
    pub artifact: ArtifactRow,
    pub session: Option<SessionRow>,
    pub entry: Option<EntryRow>,
}

pub type DocumentView = ArtifactView;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnchorView {
    pub anchor: AnchorRow,
    pub entry: Option<EntryRow>,
    pub artifact: Option<ArtifactRow>,
    pub session: Option<SessionRow>,
}

pub type EvidenceView = AnchorView;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaseRecord {
    pub case_id: String,
    pub workspace_root: Option<String>,
    pub problem: String,
    pub root_cause: Option<String>,
    pub resolution: Option<String>,
    pub commands: Vec<String>,
    #[serde(default)]
    pub verification: Vec<VerificationSummary>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchDocsRow {
    pub doc_id: String,
    pub doc_kind: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub title: Option<String>,
    pub body: String,
    #[serde(default = "empty_object")]
    pub metadata_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthGrantRecord {
    pub workspace_id: String,
    pub token_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSnapshot {
    pub schema_version: String,
    pub grants: Vec<AuthGrantRecord>,
    #[serde(rename = "admin_tokens")]
    pub admin_token_sha256s: Vec<String>,
}

impl AuthSnapshot {
    pub fn empty() -> Self {
        Self {
            schema_version: KERNEL_SCHEMA_VERSION.to_string(),
            grants: Vec::new(),
            admin_token_sha256s: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceTokenPlan {
    pub workspace_id: String,
    pub token_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminTokenPlan {
    pub token_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub db_path: String,
    pub schema_version: String,
    pub ingress_receipts: usize,
    pub sessions: usize,
    pub entries: usize,
    pub episodes: usize,
    pub insights: usize,
    pub verifications: usize,
    pub claims: usize,
    pub procedures: usize,
    pub pending_projection_count: usize,
    pub pending_derived_count: usize,
    pub pending_index_count: usize,
}

pub fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}
