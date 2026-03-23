use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use axiomsync_domain::domain::{
    AdminTokenPlan, AppendRawEventsRequest, DerivePlan, IngestPlan, ProjectionPlan, ReplayPlan,
    SearchClaimsRequest, SearchDocsRequest, SearchEntriesRequest, SearchEpisodesRequest,
    SearchInsightsRequest, SearchProceduresRequest, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest, WorkspaceTokenPlan,
};
use axiomsync_kernel::AxiomSync;

const CLI_AFTER_HELP: &str = "\
Quick start:
  axiomsync init
  axiomsync sink plan-append-raw-events --file raw-events.json > ingest-plan.json
  axiomsync sink apply-ingest-plan --file ingest-plan.json
  axiomsync project plan-rebuild > replay-plan.json
  axiomsync project apply-replay-plan --file replay-plan.json
  axiomsync project doctor

See `axiomsync sink <command> --help`, `axiomsync project <command> --help`, and `axiomsync query <command> --help` for request JSON examples.";

const PLAN_APPEND_RAW_EVENTS_AFTER_HELP: &str = r#"Input JSON example:
{
  "batch_id": "relay-2026-03-23T12:00:00Z-001",
  "source": {
    "source_kind": "axiomrelay",
    "connector_name": "chatgpt_web_selection"
  },
  "events": [
    {
      "native_session_id": "chatgpt:abc123",
      "native_entry_id": "msg_42",
      "event_type": "selection_captured",
      "captured_at_ms": 1710000000000,
      "observed_at_ms": 1710000000123,
      "payload": {
        "selection": {
          "text": "Use a narrow sink contract between relayd and AxiomSync."
        }
      },
      "hints": {
        "session_kind": "conversation",
        "entry_kind": "message"
      }
    }
  ]
}

Writes an ingest plan JSON document to stdout."#;

const APPLY_INGEST_PLAN_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `plan-append-raw-events`.";

const PLAN_SOURCE_CURSOR_AFTER_HELP: &str = r#"Input JSON example:
{
  "source": "codex",
  "cursor": {
    "cursor_key": "events",
    "cursor_value": "cursor-1",
    "updated_at_ms": 1710000000000,
    "metadata": {
      "checkpoint": "spool-offset-1"
    }
  }
}

Writes a source cursor upsert plan JSON document to stdout."#;

const APPLY_SOURCE_CURSOR_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `plan-upsert-source-cursor`.";

const PLAN_PROJECTION_AFTER_HELP: &str = "Builds a projection plan from the current raw ledger and writes the serialized plan JSON to stdout.";

const APPLY_PROJECTION_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `project plan-projection`.";

const PLAN_DERIVATIONS_AFTER_HELP: &str = "Builds a derivation plan from the current projected rows and writes the serialized plan JSON to stdout.";

const APPLY_DERIVATION_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `project plan-derivations`.";

const PLAN_REBUILD_AFTER_HELP: &str = "Builds a replay plan that contains both projection and derivation work and writes the serialized plan JSON to stdout.";

const APPLY_REPLAY_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `project plan-rebuild`.";

const SEARCH_AFTER_HELP: &str = r#"Input JSON example:
{
  "query": "narrow sink contract",
  "limit": 10,
  "filter": {
    "workspace_root": "/workspace/demo"
  }
}"#;

#[derive(Debug, Parser)]
#[command(name = "axiomsync")]
#[command(about = "AxiomSync knowledge kernel")]
#[command(after_help = CLI_AFTER_HELP)]
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
    #[command(after_long_help = PLAN_APPEND_RAW_EVENTS_AFTER_HELP)]
    PlanAppendRawEvents(FileArg),
    #[command(after_long_help = APPLY_INGEST_PLAN_AFTER_HELP)]
    ApplyIngestPlan(FileArg),
    #[command(after_long_help = PLAN_SOURCE_CURSOR_AFTER_HELP)]
    PlanUpsertSourceCursor(FileArg),
    #[command(after_long_help = APPLY_SOURCE_CURSOR_AFTER_HELP)]
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
    #[command(after_long_help = PLAN_PROJECTION_AFTER_HELP)]
    PlanProjection,
    #[command(after_long_help = APPLY_PROJECTION_AFTER_HELP)]
    ApplyProjectionPlan {
        #[arg(long)]
        file: PathBuf,
    },
    #[command(after_long_help = PLAN_DERIVATIONS_AFTER_HELP)]
    PlanDerivations,
    #[command(after_long_help = APPLY_DERIVATION_AFTER_HELP)]
    ApplyDerivationPlan {
        #[arg(long)]
        file: PathBuf,
    },
    #[command(after_long_help = PLAN_REBUILD_AFTER_HELP)]
    PlanRebuild,
    #[command(after_long_help = APPLY_REPLAY_AFTER_HELP)]
    ApplyReplayPlan {
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
pub struct QueryArgs {
    #[command(subcommand)]
    pub command: QueryCommand,
}

#[derive(Debug, Subcommand)]
pub enum QueryCommand {
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchEntries(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchEpisodes(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchDocs(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchInsights(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchClaims(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    SearchProcedures(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    FindFix(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    FindDecision(SearchFileArg),
    #[command(after_long_help = SEARCH_AFTER_HELP)]
    FindRunbook(SearchFileArg),
    GetEvidenceBundle(EvidenceBundleArg),
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
pub struct EvidenceBundleArg {
    #[arg(long)]
    pub subject_kind: String,
    #[arg(long)]
    pub subject_id: String,
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
                print_json(&serde_json::to_value(
                    app.plan_source_cursor_upsert(request)?,
                )?)?;
            }
            SinkCommand::ApplySourceCursorPlan(file) => {
                let plan: SourceCursorUpsertPlan = load_json_file(&file.file)?;
                print_json(&app.apply_source_cursor_plan(&plan)?)?;
            }
        },
        Command::Project(args) => match args.command {
            ProjectCommand::PlanProjection => {
                print_json(&serde_json::to_value(app.build_projection_plan()?)?)?;
            }
            ProjectCommand::ApplyProjectionPlan { file } => {
                let plan: ProjectionPlan = load_json_file(&file)?;
                print_json(&app.apply_projection_plan(&plan)?)?;
            }
            ProjectCommand::PlanDerivations => {
                print_json(&serde_json::to_value(app.build_derivation_plan()?)?)?;
            }
            ProjectCommand::ApplyDerivationPlan { file } => {
                let plan: DerivePlan = load_json_file(&file)?;
                print_json(&app.apply_derivation_plan(&plan)?)?;
            }
            ProjectCommand::PlanRebuild => {
                print_json(&serde_json::to_value(app.build_replay_plan()?)?)?;
            }
            ProjectCommand::ApplyReplayPlan { file } => {
                let plan: ReplayPlan = load_json_file(&file)?;
                print_json(&app.apply_replay(&plan)?)?;
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
            QueryCommand::SearchDocs(file) => {
                let request: SearchDocsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_docs(request)?)?)?;
            }
            QueryCommand::SearchInsights(file) => {
                let request: SearchInsightsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_insights(request)?)?)?;
            }
            QueryCommand::SearchClaims(file) => {
                let request: SearchClaimsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_claims(request)?)?)?;
            }
            QueryCommand::SearchProcedures(file) => {
                let request: SearchProceduresRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_procedures(request)?)?)?;
            }
            QueryCommand::FindFix(file) => {
                let request: SearchInsightsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.find_fix(request)?)?)?;
            }
            QueryCommand::FindDecision(file) => {
                let request: SearchInsightsRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.find_decision(request)?)?)?;
            }
            QueryCommand::FindRunbook(file) => {
                let request: SearchProceduresRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.find_runbook(request)?)?)?;
            }
            QueryCommand::GetEvidenceBundle(arg) => {
                print_json(&serde_json::to_value(
                    app.get_evidence_bundle(&arg.subject_kind, &arg.subject_id)?,
                )?)?;
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
                McpTransport::Stdio => {
                    runtime()?.block_on(axiomsync_mcp::serve_stdio(app, workspace_id.as_deref()))?
                }
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
