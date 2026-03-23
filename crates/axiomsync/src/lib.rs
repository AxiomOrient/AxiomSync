use std::path::PathBuf;
use std::sync::Arc;

pub mod error {
    pub use axiomsync_domain::error::*;
}

pub mod domain {
    pub use axiomsync_domain::domain::*;
}

pub mod context_db {
    pub use axiomsync_store_sqlite::context_db::*;
}

pub mod auth_store;
pub mod llm;
pub mod logic {
    pub use axiomsync_kernel::logic::*;
}
pub mod kernel {
    pub use axiomsync_kernel::kernel::*;
}
pub mod ports {
    pub use axiomsync_kernel::ports::*;
}
pub mod mcp {
    pub use axiomsync_mcp::*;
}

pub mod command_line;
pub mod http_api;
pub mod sink;
pub mod web_ui;

pub use axiomsync_domain::{AxiomError, Result};
pub use axiomsync_kernel::AxiomSync;

pub(crate) fn print_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub(crate) fn build_derivation_plan(app: &AxiomSync) -> Result<domain::DerivePlan> {
    let inputs = app.load_derivation_inputs()?;
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_derivation(&inputs, &enrichment)
}

pub(crate) fn build_replay_plan(app: &AxiomSync) -> Result<domain::ReplayPlan> {
    let raw_events = app.load_raw_events()?;
    let projection = app.plan_projection(&raw_events)?;
    let inputs = app.derivation_inputs_from_projection(&projection);
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_replay(&raw_events, &enrichment)
}

pub(crate) fn build_purge_plan(
    app: &AxiomSync,
    source: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<domain::PurgePlan> {
    let raw_events = app.load_raw_events()?;
    let mut surviving = Vec::new();
    for event in &raw_events {
        if !logic::raw_event_matches_purge(event, source, workspace_id)? {
            surviving.push(event.clone());
        }
    }
    let projection = app.plan_projection(&surviving)?;
    let inputs = app.derivation_inputs_from_projection(&projection);
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_purge(&raw_events, source, workspace_id, &enrichment)
}

pub fn open(root: impl Into<PathBuf>) -> Result<AxiomSync> {
    open_with_llm(root, llm::default_llm_client())
}

pub fn with_llm(
    root: impl Into<PathBuf>,
    llm: ports::SharedLlmExtractionPort,
) -> Result<AxiomSync> {
    open_with_llm(root, llm)
}

pub fn open_with_llm(
    root: impl Into<PathBuf>,
    llm: ports::SharedLlmExtractionPort,
) -> Result<AxiomSync> {
    let root = root.into();
    let repo = Arc::new(context_db::ContextDb::open(root.clone())?) as ports::SharedRepositoryPort;
    let auth = Arc::new(auth_store::AuthStore::open(root)?) as ports::SharedAuthStorePort;
    Ok(AxiomSync::new(repo, auth, llm))
}
