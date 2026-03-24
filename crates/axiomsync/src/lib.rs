use std::path::PathBuf;
use std::sync::Arc;

pub mod error {
    pub use axiomsync_domain::error::*;
}

pub mod domain {
    pub use axiomsync_domain::*;
}

pub mod context_db {
    pub use axiomsync_store_sqlite::context_db::*;
}

pub mod auth_store;
pub mod command_line;
pub mod http_api;

pub mod kernel {
    pub use axiomsync_kernel::kernel::*;
}

pub mod ports {
    pub use axiomsync_kernel::ports::*;
}

pub mod mcp {
    pub use axiomsync_mcp::*;
}

pub use axiomsync_domain::{AxiomError, Result};
pub use axiomsync_kernel::AxiomSync;

pub fn open(root: impl Into<PathBuf>) -> Result<AxiomSync> {
    let root = root.into();
    let repo = Arc::new(context_db::ContextDb::open(root.clone())?) as ports::SharedRepositoryPort;
    let auth = Arc::new(auth_store::AuthStore::open(root)?) as ports::SharedAuthStorePort;
    Ok(AxiomSync::new(repo, auth))
}
