use super::*;

pub(super) fn apply_batch(app: AxiomSync, batch: ConnectorBatchInput, dry_run: bool) -> Result<()> {
    let plan = app.plan_ingest(&batch)?;
    if dry_run {
        print_json(&serde_json::to_value(plan)?)?;
    } else {
        print_json(&serde_json::json!({
            "plan": plan,
            "applied": app.apply_ingest(&plan)?,
        }))?;
    }
    Ok(())
}

pub(super) fn build_runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
