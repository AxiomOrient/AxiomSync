use super::*;

impl ConnectorAdapter {
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
}
