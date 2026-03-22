use super::*;

mod ingest_io;
mod serve;
mod sync;
mod watch;

#[derive(Debug, Clone)]
pub enum ConnectorAdapter {
    Chatgpt,
    Codex,
    ClaudeCode,
    GeminiCli,
    Custom(String),
}

impl ConnectorAdapter {
    pub fn from_connector_name(name: ConnectorName) -> Self {
        match name {
            ConnectorName::Chatgpt => Self::Chatgpt,
            ConnectorName::Codex => Self::Codex,
            ConnectorName::ClaudeCode => Self::ClaudeCode,
            ConnectorName::GeminiCli => Self::GeminiCli,
        }
    }

    pub fn from_connector_label(label: &str) -> Self {
        match label {
            "chatgpt" => Self::Chatgpt,
            "codex" => Self::Codex,
            "claude_code" => Self::ClaudeCode,
            "gemini_cli" => Self::GeminiCli,
            other => Self::Custom(other.to_string()),
        }
    }

    fn connector_name(&self) -> &str {
        match self {
            Self::Chatgpt => "chatgpt",
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::GeminiCli => "gemini_cli",
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl ConnectorPort for ConnectorAdapter {
    fn connector_name(&self) -> &str {
        self.connector_name()
    }

    fn parse_batch(&self, value: Value) -> Result<ConnectorBatchInput> {
        parse::parse_batch(self.connector_name(), value)
    }
}
