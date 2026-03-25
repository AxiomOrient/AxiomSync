use std::fs;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
  axiomsync-cli init
  axiomsync-cli sink plan-append-raw-events --file raw-events.json > ingest-plan.json
  axiomsync-cli sink apply-ingest-plan --file ingest-plan.json
  axiomsync-cli project plan-rebuild > replay-plan.json
  axiomsync-cli project apply-replay-plan --file replay-plan.json
  axiomsync-cli project doctor

See `axiomsync-cli sink <command> --help`, `axiomsync-cli project <command> --help`, and `axiomsync-cli query <command> --help` for request JSON examples.";

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

pub fn open(root: impl Into<PathBuf>) -> Result<AxiomSync> {
    let root = root.into();
    let repo = Arc::new(axiomsync_store_sqlite::ContextDb::open(root.clone())?)
        as axiomsync_kernel::ports::SharedRepositoryPort;
    let auth = Arc::new(AuthStore::open(root)?) as axiomsync_kernel::ports::SharedAuthStorePort;
    Ok(AxiomSync::new(repo, auth))
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

#[derive(Debug, Clone)]
struct AuthStore {
    root: PathBuf,
}

impl AuthStore {
    fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn auth_file_path(&self) -> PathBuf {
        self.root.join("auth.json")
    }

    fn read_snapshot(&self) -> Result<axiomsync_domain::AuthSnapshot> {
        match fs::read(self.auth_file_path()) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(axiomsync_domain::AuthSnapshot::empty())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn write_snapshot(&self, snapshot: &axiomsync_domain::AuthSnapshot) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(snapshot)?;
        let path = self.auth_file_path();
        #[cfg(unix)]
        {
            use std::io::Write as _;
            use std::os::unix::fs::OpenOptionsExt as _;
            let tmp_path = path.with_extension("json.tmp");
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_data()?;
            drop(file);
            fs::rename(&tmp_path, &path)?;
        }
        #[cfg(not(unix))]
        {
            use std::io::Write as _;
            let tmp_path = path.with_extension("json.tmp");
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)?;
            file.write_all(&bytes)?;
            drop(file);
            fs::rename(&tmp_path, &path)?;
        }
        Ok(())
    }
}

impl axiomsync_kernel::ports::AuthStorePort for AuthStore {
    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self) -> PathBuf {
        self.auth_file_path()
    }

    fn read(&self) -> axiomsync_domain::Result<axiomsync_domain::AuthSnapshot> {
        self.read_snapshot()
            .map_err(|error| axiomsync_domain::AxiomError::Internal(error.to_string()))
    }

    fn write(&self, snapshot: &axiomsync_domain::AuthSnapshot) -> axiomsync_domain::Result<()> {
        self.write_snapshot(snapshot)
            .map_err(|error| axiomsync_domain::AxiomError::Internal(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct NormalizedCliRunImport {
    finished_at_ms: i64,
    run_id: String,
    command_event_id: String,
    workspace_root: String,
    task_id: String,
    actor: String,
    command: axiomsync_domain::CommandPayload,
    stdout_artifact: Option<axiomsync_domain::ArtifactRef>,
    stderr_artifact: Option<axiomsync_domain::ArtifactRef>,
    changed_files: Vec<String>,
    verification: axiomsync_domain::VerificationPayload,
}

#[derive(Debug, Clone, PartialEq)]
struct NormalizedWorkStateImport {
    exported_at_ms: i64,
    snapshot_id: String,
    workspace_root: String,
    run_id: String,
    task_id: String,
    status: String,
    progress_summary: String,
    task_file_uri: String,
    result_file_uri: String,
    events_file_uri: String,
    evidence_uris: Vec<String>,
}

fn compile_cli_run_import(payload: CliCommandPayload) -> Result<AppendRawEventsRequest> {
    let normalized = normalize_cli_run_import(payload)?;
    Ok(plan_cli_run_import(normalized))
}

fn compile_work_state_import(payload: WorkStateExportPayload) -> Result<AppendRawEventsRequest> {
    let normalized = normalize_work_state_import(payload)?;
    Ok(plan_work_state_import(normalized))
}

fn normalize_cli_run_import(payload: CliCommandPayload) -> Result<NormalizedCliRunImport> {
    payload.validate()?;
    Ok(NormalizedCliRunImport {
        finished_at_ms: millis_to_i64(payload.finished_at_ms, "finished_at_ms")?,
        run_id: payload.run_id,
        command_event_id: payload.command_event_id,
        workspace_root: payload.workspace_root,
        task_id: payload.task_id,
        actor: payload.actor,
        command: payload.command,
        stdout_artifact: payload.stdout_artifact,
        stderr_artifact: payload.stderr_artifact,
        changed_files: payload.changed_files,
        verification: payload.verification,
    })
}

fn normalize_work_state_import(payload: WorkStateExportPayload) -> Result<NormalizedWorkStateImport> {
    payload.validate()?;
    Ok(NormalizedWorkStateImport {
        exported_at_ms: millis_to_i64(payload.exported_at_ms, "exported_at_ms")?,
        snapshot_id: payload.snapshot_id,
        workspace_root: payload.workspace_root,
        run_id: payload.run_id,
        task_id: payload.task_id,
        status: payload.status,
        progress_summary: payload.progress_summary,
        task_file_uri: payload.task_file_uri,
        result_file_uri: payload.result_file_uri,
        events_file_uri: payload.events_file_uri,
        evidence_uris: payload.evidence_uris,
    })
}

fn plan_cli_run_import(normalized: NormalizedCliRunImport) -> AppendRawEventsRequest {
    AppendRawEventsRequest {
        batch_id: format!("cli-run-{}", normalized.command_event_id),
        producer: "axiomsync-cli".to_string(),
        received_at_ms: normalized.finished_at_ms,
        events: vec![axiomsync_domain::RawEventInput {
            connector: "cli_local_exec".to_string(),
            native_schema_version: Some("1".to_string()),
            session_kind: Some("run".to_string()),
            external_session_key: Some(format!("run:{}", normalized.run_id)),
            external_entry_key: Some(normalized.command_event_id.clone()),
            event_kind: Some("command_finished".to_string()),
            observed_at: None,
            captured_at: None,
            workspace_root: Some(normalized.workspace_root.clone()),
            content_hash: None,
            dedupe_key: None,
            ts_ms: Some(normalized.finished_at_ms),
            observed_at_ms: None,
            captured_at_ms: None,
            payload: json!({
                "session_kind": "run",
                "workspace_root": normalized.workspace_root,
                "task_id": normalized.task_id,
                "actor": normalized.actor,
                "command": normalized.command,
                "stdout_artifact": normalized.stdout_artifact,
                "stderr_artifact": normalized.stderr_artifact,
                "changed_files": normalized.changed_files,
                "verification": normalized.verification,
            }),
            raw_payload: None,
            artifacts: Vec::new(),
            hints: json!({}),
        }],
    }
}

fn plan_work_state_import(normalized: NormalizedWorkStateImport) -> AppendRawEventsRequest {
    AppendRawEventsRequest {
        batch_id: format!("work-state-{}", normalized.snapshot_id),
        producer: "axiomsync-cli".to_string(),
        received_at_ms: normalized.exported_at_ms,
        events: vec![axiomsync_domain::RawEventInput {
            connector: "work_state_export".to_string(),
            native_schema_version: Some("1".to_string()),
            session_kind: Some("task".to_string()),
            external_session_key: Some(format!("task:{}", normalized.task_id)),
            external_entry_key: Some(normalized.snapshot_id.clone()),
            event_kind: Some("task_state_imported".to_string()),
            observed_at: None,
            captured_at: None,
            workspace_root: Some(normalized.workspace_root.clone()),
            content_hash: None,
            dedupe_key: None,
            ts_ms: Some(normalized.exported_at_ms),
            observed_at_ms: None,
            captured_at_ms: None,
            payload: json!({
                "session_kind": "task",
                "workspace_root": normalized.workspace_root,
                "run_id": normalized.run_id,
                "task_id": normalized.task_id,
                "status": normalized.status,
                "progress_summary": normalized.progress_summary,
                "task_file_uri": normalized.task_file_uri,
                "result_file_uri": normalized.result_file_uri,
                "events_file_uri": normalized.events_file_uri,
                "evidence_uris": normalized.evidence_uris,
            }),
            raw_payload: None,
            artifacts: Vec::new(),
            hints: json!({}),
        }],
    }
}

fn millis_to_i64(value: u64, field: &str) -> Result<i64> {
    Ok(i64::try_from(value).map_err(|_| {
        axiomsync_domain::AxiomError::Validation(format!(
            "{field} exceeds supported timestamp range"
        ))
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_run_import_rejects_timestamp_overflow() {
        let payload = CliCommandPayload {
            run_id: "run-1".to_string(),
            command_event_id: "evt-1".to_string(),
            workspace_root: "/workspace/demo".to_string(),
            task_id: "task-1".to_string(),
            actor: "assistant".to_string(),
            command: axiomsync_domain::CommandPayload {
                argv: vec!["cargo".to_string(), "test".to_string()],
                cwd: "/workspace/demo".to_string(),
                exit_code: 0,
                duration_ms: 1,
                env_keys: Vec::new(),
            },
            stdout_artifact: None,
            stderr_artifact: None,
            changed_files: Vec::new(),
            verification: axiomsync_domain::VerificationPayload {
                kind: "command".to_string(),
                status: "passed".to_string(),
                summary: None,
            },
            finished_at_ms: u64::MAX,
        };

        let error = compile_cli_run_import(payload).expect_err("overflow should fail");
        assert!(
            error
                .to_string()
                .contains("finished_at_ms exceeds supported timestamp range")
        );
    }

    #[test]
    fn work_state_import_rejects_timestamp_overflow() {
        let payload = WorkStateExportPayload {
            snapshot_id: "snap-1".to_string(),
            exported_at_ms: u64::MAX,
            workspace_root: "/workspace/demo".to_string(),
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            status: "running".to_string(),
            progress_summary: "in progress".to_string(),
            task_file_uri: "file:///workspace/demo/task.md".to_string(),
            result_file_uri: "file:///workspace/demo/result.md".to_string(),
            events_file_uri: "file:///workspace/demo/events.json".to_string(),
            evidence_uris: Vec::new(),
        };

        let error = compile_work_state_import(payload).expect_err("overflow should fail");
        assert!(
            error
                .to_string()
                .contains("exported_at_ms exceeds supported timestamp range")
        );
    }

    #[test]
    fn cli_run_import_rejects_empty_required_fields() {
        let payload = CliCommandPayload {
            run_id: "".to_string(),
            command_event_id: "evt-1".to_string(),
            workspace_root: "/workspace/demo".to_string(),
            task_id: "task-1".to_string(),
            actor: "assistant".to_string(),
            command: axiomsync_domain::CommandPayload {
                argv: vec!["cargo".to_string(), "test".to_string()],
                cwd: "/workspace/demo".to_string(),
                exit_code: 0,
                duration_ms: 1,
                env_keys: Vec::new(),
            },
            stdout_artifact: None,
            stderr_artifact: None,
            changed_files: Vec::new(),
            verification: axiomsync_domain::VerificationPayload {
                kind: "command".to_string(),
                status: "passed".to_string(),
                summary: None,
            },
            finished_at_ms: 1710000000000,
        };

        let error = compile_cli_run_import(payload).expect_err("empty run_id should fail");
        assert!(
            error.to_string().contains("run_id"),
            "error should mention the field: {error}"
        );
    }

    #[test]
    fn work_state_import_rejects_empty_required_fields() {
        let payload = WorkStateExportPayload {
            snapshot_id: "snap-1".to_string(),
            exported_at_ms: 1710000000000,
            workspace_root: "/workspace/demo".to_string(),
            run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            status: "".to_string(),
            progress_summary: "in progress".to_string(),
            task_file_uri: "file:///workspace/demo/task.md".to_string(),
            result_file_uri: "file:///workspace/demo/result.md".to_string(),
            events_file_uri: "file:///workspace/demo/events.json".to_string(),
            evidence_uris: Vec::new(),
        };

        let error =
            compile_work_state_import(payload).expect_err("empty status should fail");
        assert!(
            error.to_string().contains("status"),
            "error should mention the field: {error}"
        );
    }

    #[test]
    fn cli_run_import_normalize_and_plan_are_deterministic() {
        let payload = CliCommandPayload {
            run_id: "run-1".to_string(),
            command_event_id: "evt-1".to_string(),
            workspace_root: "/workspace/demo".to_string(),
            task_id: "task-1".to_string(),
            actor: "assistant".to_string(),
            command: axiomsync_domain::CommandPayload {
                argv: vec!["cargo".to_string(), "test".to_string()],
                cwd: "/workspace/demo".to_string(),
                exit_code: 0,
                duration_ms: 1,
                env_keys: Vec::new(),
            },
            stdout_artifact: None,
            stderr_artifact: None,
            changed_files: vec!["src/lib.rs".to_string()],
            verification: axiomsync_domain::VerificationPayload {
                kind: "command".to_string(),
                status: "passed".to_string(),
                summary: Some("ok".to_string()),
            },
            finished_at_ms: 1710000000000,
        };

        let normalized = normalize_cli_run_import(payload).expect("normalized");
        assert_eq!(normalized.finished_at_ms, 1710000000000);
        let planned = plan_cli_run_import(normalized);
        assert_eq!(planned.batch_id, "cli-run-evt-1");
        assert_eq!(planned.received_at_ms, 1710000000000);
        assert_eq!(planned.events[0].external_session_key.as_deref(), Some("run:run-1"));
    }
}
