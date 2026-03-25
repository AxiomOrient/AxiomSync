pub mod error {
    pub use axiomsync_domain::error::*;
}

pub mod domain {
    pub use axiomsync_domain::*;
}

mod derive;
mod ingest;
pub mod kernel;
mod logic;
mod mcp;
pub mod ports;
mod projection;
mod query;

pub use axiomsync_domain::{AxiomError, Result};
pub use kernel::AxiomSync;

/// Prefix used in `AxiomError::Validation` messages when a tool name is unknown.
/// Exported so MCP transport layers can map this to JSON-RPC -32601 without
/// coupling to the literal error string.
pub const UNKNOWN_TOOL_ERROR_PREFIX: &str = "unknown tool ";
