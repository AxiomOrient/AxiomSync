use serde::{Deserialize, Serialize};

use crate::uri::AxiomUri;

use super::{Kind, NamespaceKey, RetentionClass};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_id: String,
    pub uri: AxiomUri,
    pub namespace: NamespaceKey,
    pub kind: Kind,
    pub event_time: i64,
    pub title: Option<String>,
    pub summary_text: Option<String>,
    pub severity: Option<String>,
    pub actor_uri: Option<AxiomUri>,
    pub subject_uri: Option<AxiomUri>,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Vec<String>,
    pub attrs: serde_json::Value,
    pub object_uri: Option<AxiomUri>,
    pub content_hash: Option<String>,
    pub tombstoned_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEventRequest {
    pub event_id: String,
    pub uri: AxiomUri,
    pub namespace: NamespaceKey,
    pub kind: Kind,
    pub event_time: i64,
    pub title: Option<String>,
    pub summary_text: Option<String>,
    pub severity: Option<String>,
    pub actor_uri: Option<AxiomUri>,
    pub subject_uri: Option<AxiomUri>,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub attrs: serde_json::Value,
    pub object_uri: Option<AxiomUri>,
    pub content_hash: Option<String>,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<NamespaceKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_tombstoned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventArchiveReport {
    pub archive_id: String,
    pub event_count: usize,
    pub namespace_prefix: Option<NamespaceKey>,
    pub kind: Option<Kind>,
    pub retention: RetentionClass,
    pub object_uri: AxiomUri,
    pub exported_at: i64,
}
