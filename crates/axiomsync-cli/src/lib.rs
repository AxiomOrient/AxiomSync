use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;

use axiomsync_domain::{
    AdminTokenPlan, AppendRawEventsRequest, CliCommandPayload, DerivePlan, IngestPlan,
    ProjectionPlan, ReplayPlan, SearchCasesRequest, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest, WorkStateExportPayload, WorkspaceTokenPlan,
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
  "producer": "axiomrelay",
  "received_at_ms": 1710000000123,
  "events": [
    {
      "connector": "chatgpt_web_selection",
      "native_schema_version": "1",
      "native_session_id": "chatgpt:abc123",
      "native_entry_id": "msg_42",
      "event_type": "selection_captured",
      "ts_ms": 1710000000123,
      "payload": {
        "session_kind": "thread",
        "selection": {
          "text": "Use a narrow sink contract between relayd and AxiomSync."
        }
      }
    }
  ]
}

Writes an ingest plan JSON document to stdout."#;

const APPLY_INGEST_PLAN_AFTER_HELP: &str =
    "Input must be the JSON plan previously returned by `plan-append-raw-events`.";

const PLAN_SOURCE_CURSOR_AFTER_HELP: &str = r#"Input JSON example:
{
  "connector": "codex",
  "cursor_key": "events",
  "cursor_value": "cursor-1",
  "updated_at_ms": 1710000000000
}

Writes a source cursor upsert plan JSON document to stdout."#;

const IMPORT_CLI_RUN_AFTER_HELP: &str =
    "Compiles a CLI command payload JSON file into the canonical append_raw_events request.";

const IMPORT_WORK_STATE_AFTER_HELP: &str =
    "Compiles a work-state export JSON file into the canonical append_raw_events request.";

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
    #[command(after_long_help = IMPORT_CLI_RUN_AFTER_HELP)]
    ImportCliRun(FileArg),
    #[command(after_long_help = IMPORT_WORK_STATE_AFTER_HELP)]
    ImportWorkState(FileArg),
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
    SearchCases(SearchFileArg),
    GetCase(IdArg),
    GetThread(IdArg),
    GetRun(IdArg),
    GetTask(IdArg),
    GetDocument(IdArg),
    GetEvidence(IdArg),
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
            SinkCommand::ImportCliRun(file) => {
                let payload: CliCommandPayload = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(compile_cli_run_import(payload)?)?)?;
            }
            SinkCommand::ImportWorkState(file) => {
                let payload: WorkStateExportPayload = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(compile_work_state_import(payload)?)?)?;
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
            QueryCommand::SearchCases(file) => {
                let request: SearchCasesRequest = load_json_file(&file.file)?;
                print_json(&serde_json::to_value(app.search_cases(request)?)?)?;
            }
            QueryCommand::GetCase(id) => {
                print_json(&serde_json::to_value(app.get_case(&id.id)?)?)?;
            }
            QueryCommand::GetThread(id) => {
                print_json(&serde_json::to_value(app.get_thread(&id.id)?)?)?;
            }
            QueryCommand::GetRun(id) => {
                print_json(&serde_json::to_value(app.get_run(&id.id)?)?)?;
            }
            QueryCommand::GetTask(id) => {
                print_json(&serde_json::to_value(app.get_task(&id.id)?)?)?;
            }
            QueryCommand::GetDocument(id) => {
                print_json(&serde_json::to_value(app.get_document(&id.id)?)?)?;
            }
            QueryCommand::GetEvidence(id) => {
                print_json(&serde_json::to_value(app.get_evidence(&id.id)?)?)?;
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

fn compile_cli_run_import(payload: CliCommandPayload) -> Result<AppendRawEventsRequest> {
    payload.validate()?;
    Ok(AppendRawEventsRequest {
        batch_id: format!("cli-run-{}", payload.command_event_id),
        producer: "axiomsync-cli".to_string(),
        received_at_ms: payload.finished_at_ms as i64,
        events: vec![axiomsync_domain::RawEventInput {
            connector: "cli_local_exec".to_string(),
            native_schema_version: Some("1".to_string()),
            session_kind: Some("run".to_string()),
            external_session_key: Some(format!("run:{}", payload.run_id)),
            external_entry_key: Some(payload.command_event_id.clone()),
            event_kind: Some("command_finished".to_string()),
            observed_at: None,
            captured_at: None,
            workspace_root: Some(payload.workspace_root.clone()),
            content_hash: None,
            dedupe_key: None,
            ts_ms: Some(payload.finished_at_ms as i64),
            observed_at_ms: None,
            captured_at_ms: None,
            payload: json!({
                "session_kind": "run",
                "workspace_root": payload.workspace_root,
                "task_id": payload.task_id,
                "actor": payload.actor,
                "command": payload.command,
                "stdout_artifact": payload.stdout_artifact,
                "stderr_artifact": payload.stderr_artifact,
                "changed_files": payload.changed_files,
                "verification": payload.verification,
            }),
            raw_payload: None,
            artifacts: Vec::new(),
            hints: json!({}),
        }],
    })
}

fn compile_work_state_import(payload: WorkStateExportPayload) -> Result<AppendRawEventsRequest> {
    payload.validate()?;
    Ok(AppendRawEventsRequest {
        batch_id: format!("work-state-{}", payload.snapshot_id),
        producer: "axiomsync-cli".to_string(),
        received_at_ms: payload.exported_at_ms as i64,
        events: vec![axiomsync_domain::RawEventInput {
            connector: "work_state_export".to_string(),
            native_schema_version: Some("1".to_string()),
            session_kind: Some("task".to_string()),
            external_session_key: Some(format!("task:{}", payload.task_id)),
            external_entry_key: Some(payload.snapshot_id.clone()),
            event_kind: Some("task_state_imported".to_string()),
            observed_at: None,
            captured_at: None,
            workspace_root: Some(payload.workspace_root.clone()),
            content_hash: None,
            dedupe_key: None,
            ts_ms: Some(payload.exported_at_ms as i64),
            observed_at_ms: None,
            captured_at_ms: None,
            payload: json!({
                "session_kind": "task",
                "workspace_root": payload.workspace_root,
                "run_id": payload.run_id,
                "task_id": payload.task_id,
                "status": payload.status,
                "progress_summary": payload.progress_summary,
                "task_file_uri": payload.task_file_uri,
                "result_file_uri": payload.result_file_uri,
                "events_file_uri": payload.events_file_uri,
                "evidence_uris": payload.evidence_uris,
            }),
            raw_payload: None,
            artifacts: Vec::new(),
            hints: json!({}),
        }],
    })
}
