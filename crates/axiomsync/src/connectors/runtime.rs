use super::*;

pub(super) fn load_codex_sync_batch(app: &AxiomSync) -> Result<ConnectorBatchInput> {
    let adapter = ConnectorAdapter::Codex;
    let value = adapter.load_sync_config_value(app)?;
    adapter.parse_batch(value)
}

pub(super) fn run_gemini_watch(
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

pub(super) fn expand_home(raw: &str) -> String {
    if let Some(stripped) = raw.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{stripped}");
    }
    raw.to_string()
}
