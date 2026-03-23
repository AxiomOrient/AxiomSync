pub mod error {
    pub use axiomsync_domain::error::*;
}

pub mod domain {
    pub use axiomsync_domain::domain::*;
}

mod compat;
mod derive;
mod ingest;
pub mod kernel;
pub mod logic;
mod mcp;
pub mod ports;
mod projection;
mod query;

pub use axiomsync_domain::{AxiomError, Result};
pub use kernel::AxiomSync;
