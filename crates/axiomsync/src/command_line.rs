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

pub fn run(cli: Cli) -> Result<()> {
    let app = crate::open(cli.root)?;
    match cli.command {
        Command::Init => print_json(&app.init()?)?,
        Command::Connector(args) => run_connector(app, args)?,
        Command::Project(args) => run_project(app, args)?,
        Command::Derive(args) => run_derive(app, args)?,
        Command::Search(args) => run_search(app, args)?,
        Command::Runbook(args) => {
            print_json(&serde_json::to_value(app.get_runbook(&args.episode_id)?)?)?
        }
        Command::Mcp(args) => match args.command {
            McpCommand::Serve {
                transport,
                addr,
                workspace_id,
            } => match transport {
                McpTransport::Stdio => {
                    build_runtime()?.block_on(mcp::serve_stdio(app, workspace_id.as_deref()))?
                }
                McpTransport::Http => build_runtime()?.block_on(http_api::serve(app, addr))?,
            },
        },
        Command::Web(args) => build_runtime()?.block_on(http_api::serve(app, args.addr))?,
    }
    Ok(())
}

fn run_connector(app: AxiomSync, args: ConnectorArgs) -> Result<()> {
    match args.command {
        ConnectorCommand::Ingest(args) => {
            let adapter = ConnectorAdapter::from_connector_label(&args.connector);
            let batch = adapter.load_batch(
                args.file.as_deref(),
                args.cursor_key,
                args.cursor_value,
                args.cursor_ts_ms,
            )?;
            apply_batch(app, batch, args.dry_run)?;
        }
        ConnectorCommand::Sync(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            let batch = adapter.sync_batch(&app)?;
            apply_batch(app, batch, args.dry_run)?;
        }
        ConnectorCommand::Repair(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            let batch = adapter.repair_batch(args.dir.as_deref())?;
            let plan = app.plan_repair(&batch)?;
            if args.dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_repair(&plan)?,
                }))?;
            }
        }
        ConnectorCommand::Watch(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            adapter.watch_batch(app, args.dry_run, args.once)?;
        }
        ConnectorCommand::Serve(args) => {
            let adapter = ConnectorAdapter::from_connector_name(args.connector);
            build_runtime()?.block_on(adapter.serve_connector_ingest(app, args.addr))?;
        }
    }
    Ok(())
}

fn run_project(app: AxiomSync, args: ProjectArgs) -> Result<()> {
    match args.command {
        ProjectCommand::Rebuild { dry_run } => {
            let plan = app.plan_replay()?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_replay(&plan)?,
                }))?;
            }
        }
        ProjectCommand::Purge {
            connector,
            workspace_id,
            dry_run,
        } => {
            let plan = app.plan_purge(connector.as_deref(), workspace_id.as_deref())?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_purge(&plan)?,
                }))?;
            }
        }
        ProjectCommand::Doctor => {
            print_json(&serde_json::to_value(app.doctor()?)?)?;
        }
        ProjectCommand::AuthGrant {
            workspace_root,
            token,
            dry_run,
        } => {
            let plan = app.plan_workspace_token_grant(&workspace_root, &token)?;
            if dry_run {
                print_json(&serde_json::to_value(plan)?)?;
            } else {
                print_json(&serde_json::json!({
                    "plan": plan,
                    "applied": app.apply_workspace_token_grant(&plan)?,
                }))?;
            }
        }
    }
    Ok(())
}

fn run_derive(app: AxiomSync, args: DeriveArgs) -> Result<()> {
    let plan = app.plan_derivation()?;
    if args.dry_run {
        print_json(&serde_json::to_value(plan)?)?;
    } else {
        print_json(&serde_json::json!({
            "plan": plan,
            "applied": app.apply_derivation(&plan)?,
        }))?;
    }
    Ok(())
}

fn run_search(app: AxiomSync, args: SearchArgs) -> Result<()> {
    if args.commands {
        return Ok(print_json(&serde_json::to_value(
            app.search_commands(&args.query, args.limit)?,
        )?)?);
    }
    let rows = app.search_episodes(SearchEpisodesRequest {
        query: args.query,
        limit: args.limit,
        filter: SearchEpisodesFilter {
            connector: args.connector,
            workspace_id: args.workspace_id,
            status: args
                .status
                .as_deref()
                .map(EpisodeStatus::parse)
                .transpose()?,
        },
    })?;
    Ok(print_json(&serde_json::to_value(rows)?)?)
}

fn apply_batch(app: AxiomSync, batch: ConnectorBatchInput, dry_run: bool) -> Result<()> {
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

fn build_runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
