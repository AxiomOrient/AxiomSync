use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

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
#[command(about = "Explicit plan/apply AxiomSync kernel")]
pub struct Cli {
    #[arg(long, default_value = ".axiomsync")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    #[command(about = "Canonical raw-only kernel sink surface")]
    Sink(SinkArgs),
    Project(ProjectArgs),
    Derive(DeriveArgs),
    Search(SearchArgs),
    Runbook(RunbookArgs),
    Mcp(McpArgs),
    Web(WebArgs),
}

#[derive(Debug, Args)]
#[command(about = "Canonical raw-only kernel sink surface")]
pub struct SinkArgs {
    #[command(subcommand)]
    pub command: SinkCommand,
}

#[derive(Debug, Subcommand)]
pub enum SinkCommand {
    PlanAppendRawEvents(SinkFileArgs),
    ApplyIngestPlan(SinkFileArgs),
    PlanUpsertSourceCursor(SinkFileArgs),
    ApplySourceCursorPlan(SinkFileArgs),
}

#[derive(Debug, Args)]
pub struct SinkFileArgs {
    #[arg(long)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    PlanRebuild,
    ApplyReplayPlan {
        #[arg(long)]
        file: PathBuf,
    },
    PlanPurge {
        #[arg(long, alias = "connector")]
        source: Option<String>,
        #[arg(long)]
        workspace_id: Option<String>,
    },
    ApplyPurgePlan {
        #[arg(long)]
        file: PathBuf,
    },
    Doctor,
    PlanAuthGrant {
        #[arg(long)]
        workspace_root: String,
        #[arg(long)]
        token: String,
    },
    PlanAdminGrant {
        #[arg(long)]
        token: String,
    },
    ApplyAuthGrantPlan {
        #[arg(long)]
        file: PathBuf,
    },
    ApplyAdminGrantPlan {
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct DeriveArgs {
    #[command(subcommand)]
    pub command: DeriveCommand,
}

#[derive(Debug, Subcommand)]
pub enum DeriveCommand {
    Plan,
    ApplyPlan {
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long, alias = "connector")]
    pub source: Option<String>,
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
