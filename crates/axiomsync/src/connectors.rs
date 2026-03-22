use std::fs;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

use crate::domain::{ConnectorBatchInput, CursorInput, RawEventInput};
use crate::error::{AxiomError, Result};
use crate::command_line::ConnectorName;
use crate::print_json;
use crate::http_api;
use crate::kernel::AxiomSync;
use crate::logic::deterministic_directory_cursor;
use crate::ports::ConnectorPort;

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

    pub fn load_batch(
        &self,
        file: Option<&Path>,
        cursor_key: Option<String>,
        cursor_value: Option<String>,
        cursor_ts_ms: Option<i64>,
    ) -> Result<ConnectorBatchInput> {
        let content = if let Some(file) = file {
            fs::read_to_string(file)?
        } else {
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content)?;
            content
        };
        let value: Value = serde_json::from_str(&content)?;
        let mut batch = self.parse_batch(value)?;
        if let Some(cursor_key) = cursor_key {
            batch.cursor = Some(CursorInput {
                cursor_key,
                cursor_value: cursor_value.unwrap_or_default(),
                updated_at_ms: cursor_ts_ms.unwrap_or_default(),
            });
        }
        Ok(batch)
    }

    pub fn load_dir_batch(&self, dir: &Path) -> Result<ConnectorBatchInput> {
        let mut events = Vec::new();
        let mut paths = fs::read_dir(dir)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort();
        let mut latest_path = None;
        for path in paths {
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            latest_path = Some(path.display().to_string());
            let value: Value = serde_json::from_slice(&fs::read(&path)?)?;
            events.extend(self.parse_batch(value)?.events);
        }
        let cursor = deterministic_directory_cursor(&events, latest_path.as_deref());
        Ok(ConnectorBatchInput { events, cursor })
    }

    pub fn default_repair_dir(&self) -> PathBuf {
        match self {
            Self::Chatgpt | Self::ClaudeCode => PathBuf::from("."),
            Self::Codex => PathBuf::from(expand_home("~/.codex")),
            Self::GeminiCli => PathBuf::from(expand_home("~/.gemini/tmp")),
            Self::Custom(_) => PathBuf::from("."),
        }
    }

    pub fn repair_batch(&self, dir: Option<&Path>) -> Result<ConnectorBatchInput> {
        let dir = dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.default_repair_dir());
        self.load_dir_batch(&dir)
    }

    pub fn sync_batch(&self, app: &AxiomSync) -> Result<ConnectorBatchInput> {
        match self {
            Self::Codex => load_codex_sync_batch(app),
            _ => Err(AxiomError::Validation(format!(
                "sync is only supported for codex, got {}",
                self.connector_name()
            ))),
        }
    }

    pub fn watch_batch(&self, app: AxiomSync, dry_run: bool, once: bool) -> Result<()> {
        match self {
            Self::GeminiCli => run_gemini_watch(app, dry_run, once, self),
            _ => Err(AxiomError::Validation(format!(
                "watch is only supported for gemini_cli, got {}",
                self.connector_name()
            ))),
        }
    }

    pub async fn serve_connector_ingest(&self, app: AxiomSync, addr: SocketAddr) -> Result<()> {
        match self {
            Self::Chatgpt | Self::ClaudeCode => {
                http_api::serve_connector_ingest(app, addr, self.connector_name()).await
            }
            _ => Err(AxiomError::Validation(format!(
                "serve is only supported for chatgpt and claude_code, got {}",
                self.connector_name()
            ))),
        }
    }

    pub fn load_sync_config_value(&self, app: &AxiomSync) -> Result<Value> {
        match self {
            Self::Codex => {
                let config = app.load_connectors_config()?;
                let codex = config
                    .codex
                    .ok_or_else(|| AxiomError::Validation("missing [codex] config".to_string()))?;
                let mut request = Client::new().get(format!(
                    "{}/events",
                    codex.app_server_base_url.trim_end_matches('/')
                ));
                if let Some(api_key) = codex.api_key {
                    request = request.bearer_auth(api_key);
                }
                if let Some(cursor) = app
                    .source_cursors()?
                    .into_iter()
                    .find(|cursor| cursor.connector == "codex" && cursor.cursor_key == "events")
                {
                    request = request.query(&[("cursor", cursor.cursor_value)]);
                }
                let response = request
                    .send()
                    .map_err(|error| {
                        AxiomError::Internal(format!("codex sync request failed: {error}"))
                    })?
                    .error_for_status()
                    .map_err(|error| {
                        AxiomError::Internal(format!("codex sync response failed: {error}"))
                    })?;
                response.json().map_err(|error| {
                    AxiomError::Internal(format!("codex sync json decode failed: {error}"))
                })
            }
            _ => Err(AxiomError::Validation(format!(
                "sync config acquisition is only supported for codex, got {}",
                self.connector_name()
            ))),
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
        parse_batch(self.connector_name(), value)
    }
}

fn load_codex_sync_batch(app: &AxiomSync) -> Result<ConnectorBatchInput> {
    let adapter = ConnectorAdapter::Codex;
    let value = adapter.load_sync_config_value(app)?;
    adapter.parse_batch(value)
}

fn run_gemini_watch(
    app: AxiomSync,
    dry_run: bool,
    once: bool,
    adapter: &ConnectorAdapter,
) -> Result<()> {
    let config = app.load_connectors_config()?;
    let gemini = config
        .gemini_cli
        .ok_or_else(|| AxiomError::Validation("missing [gemini_cli] config".to_string()))?;
    let interval = Duration::from_millis(gemini.poll_interval_ms.unwrap_or(3_000));
    loop {
        let batch = adapter.load_dir_batch(Path::new(&expand_home(&gemini.watch_directory)))?;
        let plan = app.plan_ingest(&batch)?;
        if dry_run {
            print_json(&serde_json::to_value(plan)?)?;
        } else {
            print_json(&serde_json::json!({
                "plan": plan,
                "applied": app.apply_ingest(&plan)?,
            }))?;
        }
        if once {
            break;
        }
        thread::sleep(interval);
    }
    Ok(())
}

fn parse_batch(connector: &str, value: Value) -> Result<ConnectorBatchInput> {
    let events = if let Some(array) = value.as_array() {
        array
            .iter()
            .cloned()
            .map(|raw| parse_event(connector, raw))
            .collect::<Result<Vec<_>>>()?
    } else if let Some(events_arr) = value.get("events").and_then(Value::as_array) {
        let events = events_arr
            .iter()
            .cloned()
            .map(|raw| parse_event(connector, raw))
            .collect::<Result<Vec<_>>>()?;
        let cursor = value
            .get("cursor")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        return Ok(ConnectorBatchInput { events, cursor });
    } else {
        vec![parse_event(connector, value)?]
    };
    Ok(ConnectorBatchInput {
        events,
        cursor: None,
    })
}

fn parse_event(connector: &str, raw: Value) -> Result<RawEventInput> {
    let native_session_id = raw
        .get("native_session_id")
        .or_else(|| raw.get("conversation_id"))
        .or_else(|| raw.get("session_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation("missing native_session_id".to_string()))?;
    let event_type = raw
        .get("event_type")
        .or_else(|| raw.get("type"))
        .and_then(Value::as_str)
        .ok_or_else(|| AxiomError::Validation("missing event_type".to_string()))?;
    let ts_ms = raw
        .get("ts_ms")
        .or_else(|| raw.get("timestamp_ms"))
        .or_else(|| raw.get("timestamp"))
        .and_then(Value::as_i64)
        .unwrap_or_default();
    Ok(RawEventInput {
        connector: connector.to_string(),
        native_schema_version: raw
            .get("native_schema_version")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        native_session_id: native_session_id.to_string(),
        native_event_id: raw
            .get("native_event_id")
            .or_else(|| raw.get("message_id"))
            .or_else(|| raw.get("id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        event_type: event_type.to_string(),
        ts_ms,
        payload: raw,
    })
}

fn expand_home(raw: &str) -> String {
    if let Some(stripped) = raw.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{stripped}");
    }
    raw.to_string()
}

