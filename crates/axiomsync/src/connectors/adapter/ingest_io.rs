use super::*;

impl ConnectorAdapter {
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
            Self::Codex => PathBuf::from(runtime::expand_home("~/.codex")),
            Self::GeminiCli => PathBuf::from(runtime::expand_home("~/.gemini/tmp")),
            Self::Custom(_) => PathBuf::from("."),
        }
    }

    pub fn repair_batch(&self, dir: Option<&Path>) -> Result<ConnectorBatchInput> {
        let dir = dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.default_repair_dir());
        self.load_dir_batch(&dir)
    }
}
