use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::connectors::ConnectorAdapter;
use crate::domain::ConnectorBatchInput;
use crate::domain::{EpisodeStatus, SearchEpisodesFilter, SearchEpisodesRequest};
use crate::http_api;
use crate::kernel::AxiomSync;
use crate::mcp;
use crate::print_json;

mod dispatch;
mod runtime;

pub use dispatch::run;

#[derive(Debug, Parser)]
#[command(name = "axiomsync")]
#[command(about = "Conversation-native AxiomSync kernel")]
pub struct Cli {
    #[arg(long, default_value = ".axiomsync")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Connector(ConnectorArgs),
    Project(ProjectArgs),
    Derive(DeriveArgs),
    Search(SearchArgs),
    Runbook(RunbookArgs),
    Mcp(McpArgs),
    Web(WebArgs),
}

#[derive(Debug, Args)]
pub struct ConnectorArgs {
    #[command(subcommand)]
    pub command: ConnectorCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConnectorCommand {
    Ingest(IngestArgs),
    Sync(SyncArgs),
    Repair(RepairArgs),
    Watch(WatchArgs),
    Serve(ServeArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConnectorName {
    Chatgpt,
    Codex,
    ClaudeCode,
    GeminiCli,
}

#[derive(Debug, Args)]
pub struct IngestArgs {
    #[arg(long)]
    pub connector: String,
    #[arg(long)]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub cursor_key: Option<String>,
    #[arg(long)]
    pub cursor_value: Option<String>,
    #[arg(long)]
    pub cursor_ts_ms: Option<i64>,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[arg(value_enum)]
    pub connector: ConnectorName,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct RepairArgs {
    #[arg(value_enum)]
    pub connector: ConnectorName,
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    #[arg(value_enum)]
    pub connector: ConnectorName,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub once: bool,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(value_enum)]
    pub connector: ConnectorName,
    #[arg(long, default_value = "127.0.0.1:4402")]
    pub addr: SocketAddr,
}

#[derive(Debug, Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    Rebuild {
        #[arg(long)]
        dry_run: bool,
    },
    Purge {
        #[arg(long)]
        connector: Option<String>,
        #[arg(long)]
        workspace_id: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    AuthGrant {
        #[arg(long)]
        workspace_root: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Args)]
pub struct DeriveArgs {
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long)]
    pub connector: Option<String>,
    #[arg(long)]
    pub workspace_id: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub commands: bool,
}

#[derive(Debug, Args)]
pub struct RunbookArgs {
    pub episode_id: String,
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    Serve {
        #[arg(long, value_enum, default_value_t = McpTransport::Stdio)]
        transport: McpTransport,
        #[arg(long, default_value = "127.0.0.1:4401")]
        addr: SocketAddr,
        #[arg(long)]
        workspace_id: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Args)]
pub struct WebArgs {
    #[arg(long, default_value = "127.0.0.1:4400")]
    pub addr: SocketAddr,
}
