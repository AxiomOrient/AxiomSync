use super::*;

impl ConnectorAdapter {
    pub fn sync_batch(&self, app: &AxiomSync) -> Result<ConnectorBatchInput> {
        match self {
            Self::Codex => runtime::load_codex_sync_batch(app),
            _ => Err(AxiomError::Validation(format!(
                "sync is only supported for codex, got {}",
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
}
