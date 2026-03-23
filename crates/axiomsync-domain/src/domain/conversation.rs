use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AxiomError, Result};

use super::enums::{ItemType, SelectorType};

pub const RENEWAL_SCHEMA_VERSION: &str = "renewal-sqlite-v1";

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
    #[serde(default)]
    pub grants: Vec<AuthGrantRecord>,
    #[serde(default)]
    pub admin_tokens: Vec<String>,
}

impl AuthSnapshot {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: RENEWAL_SCHEMA_VERSION.to_string(),
            grants: Vec::new(),
            admin_tokens: Vec::new(),
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

impl SourceCursorRow {
    pub fn validate(&self) -> Result<()> {
        if self.connector.trim().is_empty()
            || self.cursor_key.trim().is_empty()
            || self.cursor_value.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "source_cursor requires connector, cursor_key, cursor_value".to_string(),
            ));
        }
        Ok(())
    }
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

impl ConvSessionRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.connector.trim().is_empty()
            || self.native_session_id.trim().is_empty()
            || self.status.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "conv_session requires stable_id, connector, native_session_id, status".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvTurnRow {
    pub stable_id: String,
    pub session_id: String,
    pub native_turn_id: Option<String>,
    pub turn_index: usize,
    pub actor: String,
}

impl ConvTurnRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty()
            || self.session_id.trim().is_empty()
            || self.actor.trim().is_empty()
        {
            return Err(AxiomError::Validation(
                "conv_turn requires stable_id, session_id, actor".to_string(),
            ));
        }
        Ok(())
    }
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

impl ArtifactRow {
    pub fn validate(&self) -> Result<()> {
        if self.stable_id.trim().is_empty() || self.item_id.trim().is_empty() {
            return Err(AxiomError::Validation(
                "artifact requires stable_id and item_id".to_string(),
            ));
        }
        Ok(())
    }
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
