use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use axiomsync_domain::domain::{
    AdminTokenPlan, AppendRawEventsRequest, IngestPlan, SearchClaimsRequest, SearchEntriesRequest,
    SearchEpisodesRequest, SearchProceduresRequest, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest, WorkspaceTokenPlan,
};
use axiomsync_kernel::AxiomSync;

#[derive(Debug, Parser)]
#[command(name = "axiomsync")]
#[command(about = "AxiomSync knowledge kernel")]
pub struct Cli {
    #[arg(long, default_value = ".axiomsync")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Sink(SinkArgs),
    Project(ProjectArgs),
    Query(QueryArgs),
    Compat(CompatArgs),
    Mcp(McpArgs),
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
pub struct SinkArgs {
    #[command(subcommand)]
    pub command: SinkCommand,
}

#[derive(Debug, Subcommand)]
pub enum SinkCommand {
    PlanAppendRawEvents(FileArg),
    ApplyIngestPlan(FileArg),
    PlanUpsertSourceCursor(FileArg),
    ApplySourceCursorPlan(FileArg),
}

#[derive(Debug, Args)]
pub struct FileArg {
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
    Rebuild,
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
pub struct QueryArgs {
    #[command(subcommand)]
    pub command: QueryCommand,
}

#[derive(Debug, Subcommand)]
pub enum QueryCommand {
    SearchEntries(SearchFileArg),
    SearchEpisodes(SearchFileArg),
    SearchClaims(SearchFileArg),
    SearchProcedures(SearchFileArg),
    GetSession(IdArg),
    GetEntry(IdArg),
    GetArtifact(IdArg),
    GetAnchor(IdArg),
    GetEpisode(IdArg),
    GetClaim(IdArg),
    GetProcedure(IdArg),
}

#[derive(Debug, Args)]
pub struct SearchFileArg {
    #[arg(long)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct IdArg {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct CompatArgs {
    #[command(subcommand)]
    pub command: CompatCommand,
}

#[derive(Debug, Subcommand)]
pub enum CompatCommand {
    GetCase(IdArg),
    GetThread(IdArg),
    GetRunbook(IdArg),
    GetTask(IdArg),
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
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:4400")]
    pub addr: SocketAddr,
}

pub fn run_with<F>(cli: Cli, open: F) -> Result<()>
where
    F: Fn(PathBuf) -> Result<AxiomSync>,
{
    let app = open(cli.root)?;
    match cli.command {
        Command::Init => {
            print_json(&app.init()?)?;
        }
        Command::Sink(args) => match args.command {
            SinkCommand::PlanAppendRawEvents(file) => {
                let request: AppendRawEventsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.plan_append_raw_events(request)?)?)?;
            }
            SinkCommand::ApplyIngestPlan(file) => {
                let plan: IngestPlan = load_json_file(&file.file)?;
                print_json(&app.apply_ingest_plan(&plan)?)?;
            }
            SinkCommand::PlanUpsertSourceCursor(file) => {
                let request: UpsertSourceCursorRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.plan_source_cursor_upsert(request)?)?)?;
            }
            SinkCommand::ApplySourceCursorPlan(file) => {
                let plan: SourceCursorUpsertPlan = load_json_file(&file.file)?;
                print_json(&app.apply_source_cursor_plan(&plan)?)?;
            }
        },
        Command::Project(args) => match args.command {
            ProjectCommand::Rebuild => {
                print_json(&app.rebuild()?)?;
            }
            ProjectCommand::Doctor => {
                print_json(&serde_json::to_value(app.doctor_report()?)?)?;
            }
            ProjectCommand::PlanAuthGrant {
                workspace_root,
                token,
            } => {
                print_json(&serde_json::to_value(
                    app.plan_workspace_token_grant(&workspace_root, &token)?,
                )?)?;
            }
            ProjectCommand::PlanAdminGrant { token } => {
                print_json(&serde_json::to_value(app.plan_admin_token_grant(&token)?)?)?;
            }
            ProjectCommand::ApplyAuthGrantPlan { file } => {
                let plan: WorkspaceTokenPlan = load_json_file(&file)?;
                print_json(&app.apply_workspace_token_grant(&plan)?)?;
            }
            ProjectCommand::ApplyAdminGrantPlan { file } => {
                let plan: AdminTokenPlan = load_json_file(&file)?;
                print_json(&app.apply_admin_token_grant(&plan)?)?;
            }
        },
        Command::Query(args) => match args.command {
            QueryCommand::SearchEntries(file) => {
                let request: SearchEntriesRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_entries(request)?)?)?;
            }
            QueryCommand::SearchEpisodes(file) => {
                let request: SearchEpisodesRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_episodes(request)?)?)?;
            }
            QueryCommand::SearchClaims(file) => {
                let request: SearchClaimsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_claims(request)?)?)?;
            }
            QueryCommand::SearchProcedures(file) => {
                let request: SearchProceduresRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_procedures(request)?)?)?;
            }
            QueryCommand::GetSession(id) => {
                print_json(&serde_json::to_value(app.get_session(&id.id)?)?)?;
            }
            QueryCommand::GetEntry(id) => {
                print_json(&serde_json::to_value(app.get_entry(&id.id)?)?)?;
            }
            QueryCommand::GetArtifact(id) => {
                print_json(&serde_json::to_value(app.get_artifact(&id.id)?)?)?;
            }
            QueryCommand::GetAnchor(id) => {
                print_json(&serde_json::to_value(app.get_anchor(&id.id)?)?)?;
            }
            QueryCommand::GetEpisode(id) => {
                print_json(&serde_json::to_value(app.get_episode(&id.id)?)?)?;
            }
            QueryCommand::GetClaim(id) => {
                print_json(&serde_json::to_value(app.get_claim(&id.id)?)?)?;
            }
            QueryCommand::GetProcedure(id) => {
                print_json(&serde_json::to_value(app.get_procedure(&id.id)?)?)?;
            }
        },
        Command::Compat(args) => match args.command {
            CompatCommand::GetCase(id) => {
                print_json(&serde_json::to_value(app.get_case(&id.id)?)?)?;
            }
            CompatCommand::GetThread(id) => {
                print_json(&serde_json::to_value(app.get_thread(&id.id)?)?)?;
            }
            CompatCommand::GetRunbook(id) => {
                print_json(&serde_json::to_value(app.get_runbook(&id.id)?)?)?;
            }
            CompatCommand::GetTask(id) => {
                print_json(&serde_json::to_value(app.get_task(&id.id)?)?)?;
            }
        },
        Command::Mcp(args) => match args.command {
            McpCommand::Serve {
                transport,
                addr,
                workspace_id,
            } => match transport {
                McpTransport::Stdio => runtime()?.block_on(axiomsync_mcp::serve_stdio(app, workspace_id.as_deref()))?,
                McpTransport::Http => runtime()?.block_on(axiomsync_http::serve(app, addr))?,
            },
        },
        Command::Serve(args) => runtime()?.block_on(axiomsync_http::serve(app, args.addr))?,
    }
    Ok(())
}

fn load_json_file<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn print_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
