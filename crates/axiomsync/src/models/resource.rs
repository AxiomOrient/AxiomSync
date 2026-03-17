use serde::{Deserialize, Serialize};

use crate::uri::AxiomUri;

use super::{Kind, NamespaceKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub resource_id: String,
    pub uri: AxiomUri,
    pub namespace: NamespaceKey,
    pub kind: Kind,
    pub title: Option<String>,
    pub mime: Option<String>,
    pub tags: Vec<String>,
    pub attrs: serde_json::Value,
    pub object_uri: Option<AxiomUri>,
    pub excerpt_text: Option<String>,
    pub content_hash: String,
    pub tombstoned_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Alias retained for call-site compatibility; `ResourceRecord` is the single write+read type.
pub type UpsertResource = ResourceRecord;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<NamespaceKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_tombstoned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMountRequest {
    pub source_path: String,
    pub target_uri: AxiomUri,
    pub namespace: NamespaceKey,
    pub kind: Kind,
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub attrs: serde_json::Value,
    #[serde(default = "default_wait_true")]
    pub wait: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMountReport {
    pub root_uri: AxiomUri,
    pub resource: ResourceRecord,
    pub queued: bool,
}

const fn default_wait_true() -> bool {
    true
}
