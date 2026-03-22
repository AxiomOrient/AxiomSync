use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::enums::{VerificationKind, VerificationStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
    pub capture_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexConnectorConfig {
    pub app_server_base_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeCodeConnectorConfig {
    pub ingest_bind_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeminiConnectorConfig {
    pub watch_directory: String,
    pub poll_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ConnectorsConfig {
    pub chatgpt: Option<ChatGptConnectorConfig>,
    pub codex: Option<CodexConnectorConfig>,
    pub claude_code: Option<ClaudeCodeConnectorConfig>,
    pub gemini_cli: Option<GeminiConnectorConfig>,
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}
