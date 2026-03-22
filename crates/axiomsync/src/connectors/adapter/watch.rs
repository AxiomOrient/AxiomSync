use super::*;

impl ConnectorAdapter {
    pub fn watch_batch(&self, app: AxiomSync, dry_run: bool, once: bool) -> Result<()> {
        match self {
            Self::GeminiCli => runtime::run_gemini_watch(app, dry_run, once, self),
            _ => Err(AxiomError::Validation(format!(
                "watch is only supported for gemini_cli, got {}",
                self.connector_name()
            ))),
        }
    }
}
