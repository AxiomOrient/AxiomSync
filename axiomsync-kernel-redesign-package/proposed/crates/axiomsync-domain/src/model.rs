use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventBatch {
    pub batch_id: String,
    pub events: Vec<RawEventEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventEnvelope {
    pub source_kind: String,
    pub connector: String,
    pub session_kind: String,
    pub external_session_key: Option<String>,
    pub external_entry_key: Option<String>,
    pub event_kind: String,
    pub observed_at: DateTime<Utc>,
    pub captured_at: Option<DateTime<Utc>>,
    pub workspace_root: Option<String>,
    pub content_hash: Option<String>,
    pub dedupe_key: Option<String>,
    pub payload: serde_json::Value,
    pub raw_payload: Option<serde_json::Value>,
    pub artifacts: Vec<RawArtifactEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawArtifactEnvelope {
    pub artifact_kind: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub session_kind: String,
    pub external_session_key: Option<String>,
    pub title: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRecord {
    pub entry_id: String,
    pub session_id: String,
    pub seq_no: i64,
    pub entry_kind: String,
    pub actor_id: Option<String>,
    pub text_body: Option<String>,
    pub metadata_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub session_id: Option<String>,
    pub episode_kind: String,
    pub summary: String,
    pub confidence: f64,
}
