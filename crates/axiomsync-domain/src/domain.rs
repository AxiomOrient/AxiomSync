use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{AxiomError, Result};

pub const RENEWAL_SCHEMA_VERSION: &str = "renewal-sqlite-v1";

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            pub fn parse(value: &str) -> Result<Self> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    other => Err(AxiomError::Validation(format!(
                        "invalid {} {}",
                        stringify!($name),
                        other
                    ))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.as_str()))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let text = value.as_str()?;
                Self::parse(text).map_err(|_| FromSqlError::Other(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid {} {}", stringify!($name), text),
                ))))
            }
        }
    };
}

string_enum!(ItemType {
    UserMsg => "user_msg",
    AssistantMsg => "assistant_msg",
    ToolCall => "tool_call",
    ToolResult => "tool_result",
    FileChange => "file_change",
    Diff => "diff",
    Plan => "plan",
});

string_enum!(SelectorType {
    TextSpan => "text_span",
    JsonPointer => "json_pointer",
    DiffHunk => "diff_hunk",
    ArtifactRange => "artifact_range",
    DomSelector => "dom_selector",
});

string_enum!(EpisodeStatus {
    Open => "open",
    Solved => "solved",
    Abandoned => "abandoned",
});

string_enum!(InsightKind {
    Problem => "problem",
    Fix => "fix",
    RootCause => "root_cause",
    Decision => "decision",
    Command => "command",
    Snippet => "snippet",
});

string_enum!(VerificationKind {
    Test => "test",
    CommandExit => "command_exit",
    DiffApplied => "diff_applied",
    HumanConfirm => "human_confirm",
});

string_enum!(VerificationStatus {
    Pass => "pass",
    Fail => "fail",
    Partial => "partial",
    Unknown => "unknown",
});

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

pub fn workspace_stable_id(canonical_root: &str) -> String {
    stable_id("ws", &canonical_root)
}

pub fn normalize_fts_query(query: &str) -> Option<String> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub stable_id: String,
    pub canonical_root: String,
    pub repo_remote: Option<String>,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
}

impl WorkspaceRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.canonical_root.trim().is_empty() {
            return Err(AxiomError::Validation(
                "workspace requires stable_id and canonical_root".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthGrantRecord {
    pub workspace_id: String,
    pub token_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AuthSnapshot {
    pub schema_version: String,
    pub grants: Vec<AuthGrantRecord>,
}

impl AuthSnapshot {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: RENEWAL_SCHEMA_VERSION.to_string(),
            grants: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawEventRow {
    pub stable_id: String,
    pub connector: String,
    pub native_schema_version: Option<String>,
    pub native_session_id: String,
    pub native_event_id: Option<String>,
    pub event_type: String,
    pub ts_ms: i64,
    pub payload_json: String,
    pub payload_sha256_hex: String,
}

impl RawEventRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.connector.trim().is_empty()
            || self.native_session_id.trim().is_empty()
            || self.event_type.trim().is_empty()
            || self.payload_sha256_hex.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "raw_event requires stable_id, connector, native_session_id, event_type, payload hash"
                    .to_string(),
            ));
        }
        let _: Value = serde_json::from_str(&self.payload_json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceCursorRow {
    pub connector: String,
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportJournalRow {
    pub stable_id: String,
    pub connector: String,
    pub imported_events: usize,
    pub skipped_events: usize,
    pub cursor_key: Option<String>,
    pub cursor_value: Option<String>,
    pub applied_at_ms: i64,
}

impl ImportJournalRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.connector.trim().is_empty() {
            return Err(AxiomError::Validation(
                "import journal requires stable_id and connector".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvSessionRow {
    pub stable_id: String,
    pub connector: String,
    pub native_session_id: String,
    pub workspace_id: Option<String>,
    pub title: Option<String>,
    pub transcript_uri: Option<String>,
    pub status: String,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvTurnRow {
    pub stable_id: String,
    pub session_id: String,
    pub native_turn_id: Option<String>,
    pub turn_index: usize,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvItemRow {
    pub stable_id: String,
    pub turn_id: String,
    pub item_type: ItemType,
    pub tool_name: Option<String>,
    pub body_text: Option<String>,
    pub payload_json: String,
}

impl ConvItemRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.turn_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "conv_item requires stable_id and turn_id".to_string(),
            ));
        }
        let _: Value = serde_json::from_str(&self.payload_json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRow {
    pub stable_id: String,
    pub item_id: String,
    pub uri: String,
    pub mime: Option<String>,
    pub sha256_hex: Option<String>,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceAnchorRow {
    pub stable_id: String,
    pub item_id: String,
    pub selector_type: SelectorType,
    pub selector_json: String,
    pub quoted_text: Option<String>,
}

impl EvidenceAnchorRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.item_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "evidence_anchor requires stable_id and item_id".to_string(),
            ));
        }
        let selector: Value = serde_json::from_str(&self.selector_json)?;
        match self.selector_type {
            SelectorType::TextSpan => validate_text_span_selector(&selector)?,
            SelectorType::JsonPointer => validate_json_pointer_selector(&selector)?,
            SelectorType::DiffHunk | SelectorType::ArtifactRange | SelectorType::DomSelector => {
                if selector.is_null() {
                    return Err(AxiomError::Validation(format!(
                        "{} selector must not be null",
                        self.selector_type
                    )));
                }
            }
        }
        Ok(())
    }
}

fn validate_text_span_selector(selector: &Value) -> Result<()> {
    let start = selector
        .get("start")
        .and_then(Value::as_u64)
        .ok_or_else(|| AxiomError::Validation("text_span selector requires start".to_string()))?;
    let end = selector
        .get("end")
        .and_then(Value::as_u64)
        .ok_or_else(|| AxiomError::Validation("text_span selector requires end".to_string()))?;
    if start > end {
        return Err(AxiomError::Validation(
            "text_span selector start must be <= end".to_string(),
        ));
    }
    Ok(())
}

fn validate_json_pointer_selector(selector: &Value) -> Result<()> {
    let pointer = selector
        .as_str()
        .or_else(|| selector.get("pointer").and_then(Value::as_str))
        .ok_or_else(|| {
            AxiomError::Validation("json_pointer selector requires pointer string".to_string())
        })?;
    if pointer != "/" && !pointer.starts_with('/') {
        return Err(AxiomError::Validation(
            "json_pointer selector must start with '/'".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeRow {
    pub stable_id: String,
    pub workspace_id: Option<String>,
    pub problem_signature: String,
    pub status: EpisodeStatus,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeMemberRow {
    pub episode_id: String,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightRow {
    pub stable_id: String,
    pub episode_id: String,
    pub kind: InsightKind,
    pub summary: String,
    pub normalized_text: String,
    pub extractor_version: String,
    pub confidence: f64,
    pub stale: bool,
}

impl InsightRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.episode_id.trim().is_empty()
            || self.summary.trim().is_empty()
            || self.extractor_version.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "insight requires stable_id, episode_id, summary, extractor_version".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(AxiomError::Validation(
                "insight confidence must be between 0 and 1".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InsightAnchorRow {
    pub insight_id: String,
    pub anchor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationRow {
    pub stable_id: String,
    pub episode_id: String,
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence_id: Option<String>,
}

impl VerificationRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.episode_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "verification requires stable_id and episode_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchDocRedactedRow {
    pub stable_id: String,
    pub episode_id: String,
    pub body: String,
}

impl SearchDocRedactedRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.episode_id.trim().is_empty()
            || self.body.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "search_doc_redacted requires stable_id, episode_id, body".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadView {
    pub session: ConvSessionRow,
    pub turns: Vec<ThreadTurnView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadTurnView {
    pub turn: ConvTurnRow,
    pub items: Vec<ThreadItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadItemView {
    pub item: ConvItemRow,
    pub artifacts: Vec<ArtifactRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceView {
    pub evidence: EvidenceAnchorRow,
    pub item: ConvItemRow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunbookRecord {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub commands: Vec<String>,
    pub verification: Vec<RunbookVerification>,
    pub evidence: Vec<String>,
}

impl RunbookRecord {
    pub fn validate(&self) -> Result<()> {
        if self.problem.trim().is_empty() {
            return Err(AxiomError::Validation(
                "runbook.problem must not be empty".to_string(),
            ));
        }
        if self
            .commands
            .iter()
            .any(|command| command.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "runbook.commands must not contain empty values".to_string(),
            ));
        }
        for verification in &self.verification {
            verification.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunbookVerification {
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence: Option<String>,
}

impl RunbookVerification {
    pub fn validate(&self) -> Result<()> {
        if let Some(evidence) = &self.evidence
            && evidence.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "runbook verification evidence must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawEventInput {
    pub connector: String,
    pub native_schema_version: Option<String>,
    pub native_session_id: String,
    pub native_event_id: Option<String>,
    pub event_type: String,
    pub ts_ms: i64,
    pub payload: Value,
}

impl RawEventInput {
    pub fn validate(&self) -> Result<()> {
        if self.connector.trim().is_empty()
            || self.native_session_id.trim().is_empty()
            || self.event_type.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "raw event input requires connector, native_session_id, event_type".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorInput {
    pub cursor_key: String,
    pub cursor_value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectorBatchInput {
    pub events: Vec<RawEventInput>,
    pub cursor: Option<CursorInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExistingRawEventKey {
    pub stable_id: String,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedRawEvent {
    pub row: RawEventRow,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestPlan {
    pub adds: Vec<NormalizedRawEvent>,
    pub cursor_update: Option<SourceCursorRow>,
    pub skipped_dedupe_keys: Vec<String>,
    pub journal: Option<ImportJournalRow>,
}

impl IngestPlan {
    pub fn validate(&self) -> Result<()> {
        for add in &self.adds {
            add.row.validate()?;
        }
        if let Some(journal) = &self.journal {
            journal.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionPlan {
    pub workspaces: Vec<WorkspaceRow>,
    pub conv_sessions: Vec<ConvSessionRow>,
    pub conv_turns: Vec<ConvTurnRow>,
    pub conv_items: Vec<ConvItemRow>,
    pub artifacts: Vec<ArtifactRow>,
    pub evidence_anchors: Vec<EvidenceAnchorRow>,
}

impl ProjectionPlan {
    pub fn validate(&self) -> Result<()> {
        for workspace in &self.workspaces {
            workspace.validate()?;
        }
        for item in &self.conv_items {
            item.validate()?;
        }
        for anchor in &self.evidence_anchors {
            anchor.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DerivePlan {
    pub episodes: Vec<EpisodeRow>,
    pub episode_members: Vec<EpisodeMemberRow>,
    pub insights: Vec<InsightRow>,
    pub insight_anchors: Vec<InsightAnchorRow>,
    pub verifications: Vec<VerificationRow>,
    pub search_docs_redacted: Vec<SearchDocRedactedRow>,
}

impl DerivePlan {
    pub fn validate(&self) -> Result<()> {
        let anchored: HashSet<_> = self
            .insight_anchors
            .iter()
            .map(|row| row.insight_id.as_str())
            .collect();
        for insight in &self.insights {
            insight.validate()?;
            if !anchored.contains(insight.stable_id.as_str()) {
                return Err(AxiomError::Validation(format!(
                    "insight {} is missing evidence anchor",
                    insight.stable_id
                )));
            }
        }
        for verification in &self.verifications {
            verification.validate()?;
        }
        for doc in &self.search_docs_redacted {
            doc.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayPlan {
    pub projection: ProjectionPlan,
    pub derivation: DerivePlan,
}

impl ReplayPlan {
    pub fn validate(&self) -> Result<()> {
        self.projection.validate()?;
        self.derivation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PurgePlan {
    pub connector: Option<String>,
    pub workspace_id: Option<String>,
    pub deleted_raw_event_ids: Vec<String>,
    pub projection: ProjectionPlan,
    pub derivation: DerivePlan,
}

impl PurgePlan {
    pub fn validate(&self) -> Result<()> {
        if self
            .connector
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "purge connector must not be empty".to_string(),
            ));
        }
        if self
            .workspace_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AxiomError::Validation(
                "purge workspace_id must not be empty".to_string(),
            ));
        }
        self.projection.validate()?;
        self.derivation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairPlan {
    pub ingest: IngestPlan,
    pub replay: ReplayPlan,
}

impl RepairPlan {
    pub fn validate(&self) -> Result<()> {
        self.ingest.validate()?;
        self.replay.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub schema_version: String,
    pub stored_schema_version: Option<String>,
    pub version_mismatch: bool,
    pub fts_rebuild_required: bool,
    pub drift_detected: bool,
    pub missing_tables: Vec<String>,
    pub missing_indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DerivationContext {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub turn_ids: Vec<String>,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
    pub transcript: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DerivationEnrichment {
    pub extractions: HashMap<String, EpisodeExtraction>,
    pub verifications: HashMap<String, Vec<VerificationExtraction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceTokenPlan {
    pub workspace_id: String,
    pub token_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchEpisodesResult {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchEpisodesRequest {
    pub query: String,
    pub limit: usize,
    pub filter: SearchEpisodesFilter,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchEpisodesFilter {
    pub connector: Option<String>,
    pub workspace_id: Option<String>,
    pub status: Option<EpisodeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchCommandsResult {
    pub episode_id: String,
    pub command: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeConnectorRow {
    pub episode_id: String,
    pub connector: Option<String>,
    pub turn_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchEpisodeFtsRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub matched_kind: Option<InsightKind>,
    pub matched_summary: Option<String>,
    pub pass_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeEvidenceSearchRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub connector: Option<String>,
    pub status: EpisodeStatus,
    pub evidence_id: String,
    pub quoted_text: Option<String>,
    pub body_text: Option<String>,
    pub pass_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchCommandCandidateRow {
    pub episode_id: String,
    pub workspace_id: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EpisodeExtraction {
    pub problem: String,
    pub root_cause: Option<String>,
    pub fix: Option<String>,
    pub commands: Vec<String>,
    pub decisions: Vec<String>,
    pub snippets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationExtraction {
    pub kind: VerificationKind,
    pub status: VerificationStatus,
    pub summary: Option<String>,
    pub evidence: Option<String>,
    pub pass_condition: Option<String>,
    pub exit_code: Option<i64>,
    pub human_confirmed: bool,
}

impl Default for VerificationExtraction {
    fn default() -> Self {
        Self {
            kind: VerificationKind::Test,
            status: VerificationStatus::Unknown,
            summary: None,
            evidence: None,
            pass_condition: None,
            exit_code: None,
            human_confirmed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatGptConnectorConfig {
    pub enabled: bool,
    pub browser_extension_id: Option<String>,
    pub poll_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexConnectorConfig {
    pub enabled: bool,
    pub app_server_base_url: String,
    pub api_key: Option<String>,
    pub repair_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeCodeConnectorConfig {
    pub enabled: bool,
    pub hooks_server_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeminiConnectorConfig {
    pub enabled: bool,
    pub watch_directory: String,
    pub poll_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorsConfig {
    pub chatgpt: Option<ChatGptConnectorConfig>,
    pub codex: Option<CodexConnectorConfig>,
    pub claude_code: Option<ClaudeCodeConnectorConfig>,
    pub gemini_cli: Option<GeminiConnectorConfig>,
}

pub fn build_search_doc_redacted(
    episode_id: &str,
    insights: &[InsightRow],
    verifications: &[VerificationRow],
) -> SearchDocRedactedRow {
    let mut sections = Vec::new();
    for kind in [
        InsightKind::Problem,
        InsightKind::RootCause,
        InsightKind::Fix,
    ] {
        for insight in insights.iter().filter(|insight| insight.kind == kind) {
            sections.push(format!("{}: {}", kind.as_str(), insight.summary));
        }
    }
    for insight in insights
        .iter()
        .filter(|insight| insight.kind == InsightKind::Command)
    {
        sections.push(format!("command: {}", insight.summary));
    }
    for verification in verifications {
        sections.push(format!(
            "verification {} {} {}",
            verification.kind,
            verification.status,
            verification.summary.clone().unwrap_or_default()
        ));
    }
    SearchDocRedactedRow {
        stable_id: stable_id("searchdoc", &episode_id),
        episode_id: episode_id.to_string(),
        body: sections.join("\n"),
    }
}
