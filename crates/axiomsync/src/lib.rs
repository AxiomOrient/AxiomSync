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
pub mod connector_config;
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
pub mod connectors;
pub mod http_api;
pub mod web_ui;

pub use axiomsync_domain::{AxiomError, Result};
pub use axiomsync_kernel::AxiomSync;

pub(crate) fn print_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
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
    let connectors = Arc::new(connector_config::FileConnectorConfigStore::new(
        repo.root().to_path_buf(),
    )) as ports::SharedConnectorConfigPort;
    Ok(AxiomSync::new(repo, auth, connectors, llm))
}
