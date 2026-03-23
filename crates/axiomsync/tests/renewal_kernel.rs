use std::sync::Arc;

use axiomsync::domain::{
    AppendRawEventsRequest, ConnectorBatchInput, ConvSessionRow, ConvTurnRow, CursorInput,
    DerivePlan, EpisodeExtraction, EpisodeRow, EpisodeStatus, ProjectionPlan, RawEventInput,
    SearchDocRedactedRow, SearchEpisodesFilter, SearchEpisodesRequest, UpsertSourceCursorRequest,
    VerificationExtraction, VerificationKind, VerificationStatus, WorkspaceRow,
};
use axiomsync::kernel::AxiomSync;
use axiomsync::llm::MockLlmClient;
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

fn plan_ingest(
    app: &AxiomSync,
    batch: &ConnectorBatchInput,
) -> axiomsync::Result<axiomsync::domain::IngestPlan> {
    let existing = app.load_existing_raw_event_keys()?;
    app.plan_ingest(&existing, batch)
}

fn plan_derivation(app: &AxiomSync) -> axiomsync::Result<axiomsync::domain::DerivePlan> {
    let inputs = app.load_derivation_inputs()?;
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_derivation(&inputs, &enrichment)
}

fn plan_replay(app: &AxiomSync) -> axiomsync::Result<axiomsync::domain::ReplayPlan> {
    let raw_events = app.load_raw_events()?;
    let projection = app.plan_projection(&raw_events)?;
    let inputs = app.derivation_inputs_from_projection(&projection);
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_replay(&raw_events, &enrichment)
}

fn plan_purge(
    app: &AxiomSync,
    source: Option<&str>,
    workspace_id: Option<&str>,
) -> axiomsync::Result<axiomsync::domain::PurgePlan> {
    let raw_events = app.load_raw_events()?;
    let mut surviving = Vec::new();
    for event in &raw_events {
        if !axiomsync::logic::raw_event_matches_purge(event, source, workspace_id)? {
            surviving.push(event.clone());
        }
    }
    let projection = app.plan_projection(&surviving)?;
    let inputs = app.derivation_inputs_from_projection(&projection);
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_purge(&raw_events, source, workspace_id, &enrichment)
}

fn plan_repair(
    app: &AxiomSync,
    batch: &ConnectorBatchInput,
) -> axiomsync::Result<axiomsync::domain::RepairPlan> {
    let ingest = plan_ingest(app, batch)?;
    let raw_events = app.load_raw_events()?;
    let mut combined = raw_events.clone();
    combined.extend(ingest.adds.iter().map(|event| event.row.clone()));
    let projection = app.plan_projection(&combined)?;
    let inputs = app.derivation_inputs_from_projection(&projection);
    let contexts = app.plan_derivation_contexts(&inputs);
    let enrichment = app.collect_derivation_enrichment(&contexts)?;
    app.plan_repair(&raw_events, &ingest, &enrichment)
}

fn mock_app() -> AxiomSync {
    let temp = tempdir().expect("tempdir");
    let root = temp.keep();
    axiomsync::open_with_llm(
        root,
        Arc::new(MockLlmClient {
            extraction: EpisodeExtraction {
                problem: "Timeout error in worker".to_string(),
                root_cause: Some("stale queue config".to_string()),
                fix: Some("reload worker config and rerun tests".to_string()),
                commands: vec!["cargo test -p axiomsync".to_string()],
                decisions: vec!["prefer sqlite context.db".to_string()],
                snippets: vec!["worker.reload();".to_string()],
            },
            verifications: vec![VerificationExtraction {
                kind: VerificationKind::Test,
                status: VerificationStatus::Pass,
                summary: Some("all tests passed".to_string()),
                evidence: Some("tests passed".to_string()),
                pass_condition: Some("mock llm".to_string()),
                exit_code: Some(0),
                human_confirmed: false,
            }],
        }),
    )
    .expect("app")
}

fn sample_batch(connector: &str) -> ConnectorBatchInput {
    ConnectorBatchInput {
        events: vec![
            RawEventInput {
                source: connector.to_string(),
                native_schema_version: Some("v1".to_string()),
                native_session_id: format!("{connector}-session"),
                native_event_id: Some("evt-1".to_string()),
                event_type: "user_message".to_string(),
                ts_ms: 1_710_000_000_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "turn_id": "turn-1",
                    "actor": "user",
                    "text": "Investigate timeout error"
                }),
            },
            RawEventInput {
                source: connector.to_string(),
                native_schema_version: Some("v1".to_string()),
                native_session_id: format!("{connector}-session"),
                native_event_id: Some("evt-2".to_string()),
                event_type: "assistant_message".to_string(),
                ts_ms: 1_710_000_001_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "turn_id": "turn-2",
                    "actor": "assistant",
                    "text": "Run the tests and inspect config"
                }),
            },
            RawEventInput {
                source: connector.to_string(),
                native_schema_version: Some("v1".to_string()),
                native_session_id: format!("{connector}-session"),
                native_event_id: Some("evt-3".to_string()),
                event_type: "tool_result".to_string(),
                ts_ms: 1_710_000_002_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "turn_id": "turn-2",
                    "actor": "tool",
                    "tool_name": "shell",
                    "text": "tests passed\nexit code: 0"
                }),
            },
            RawEventInput {
                source: connector.to_string(),
                native_schema_version: Some("v1".to_string()),
                native_session_id: format!("{connector}-session"),
                native_event_id: Some("evt-4".to_string()),
                event_type: "user_message".to_string(),
                ts_ms: 1_710_000_100_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "turn_id": "turn-3",
                    "actor": "user",
                    "text": "Now debug release timeout on another path"
                }),
            },
        ],
        cursor: Some(CursorInput {
            cursor_key: "cursor".to_string(),
            cursor_value: "4".to_string(),
            updated_at_ms: 1_710_000_003_000,
        }),
    }
}

#[test]
fn init_creates_sqlite_schema_and_extra_tables() {
    let app = mock_app();
    let report = app.init().expect("init");
    assert!(app.db_path().exists());
    assert!(report.get("auth_path").is_some(), "{report}");
    assert!(report.get("connectors_path").is_none(), "{report}");
    assert!(!app.root().join("connectors.toml").exists());

    let conn = Connection::open(app.db_path()).expect("sqlite");
    for table in [
        "workspace",
        "raw_event",
        "source_cursor",
        "import_journal",
        "conv_session",
        "conv_turn",
        "conv_item",
        "artifact",
        "evidence_anchor",
        "episode",
        "episode_member",
        "insight",
        "verification",
        "search_doc_redacted",
        "search_doc_redacted_fts",
        "insight_anchor",
        "insight_fts",
        "execution_run",
        "execution_task",
        "execution_check",
        "execution_approval",
        "execution_event",
        "document_record",
    ] {
        let exists: i64 = conn
            .query_row(
                "select count(*) from sqlite_master where name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "{table}");
    }
}

#[test]
fn ingest_project_derive_search_flow_is_deterministic() {
    let app = mock_app();

    let ingest_plan = plan_ingest(&app, &sample_batch("codex")).expect("plan ingest");
    assert_eq!(ingest_plan.adds.len(), 4);
    app.apply_ingest(&ingest_plan).expect("apply ingest");
    let conn = Connection::open(app.db_path()).expect("sqlite");
    let journal_count: i64 = conn
        .query_row("select count(*) from import_journal", [], |row| row.get(0))
        .expect("journal count");
    assert_eq!(journal_count, 1);

    let raw_events = app.load_raw_events().expect("raw events");
    let projection_plan = app.plan_projection(&raw_events).expect("projection");
    assert_eq!(projection_plan.conv_sessions.len(), 1);
    app.apply_projection(&projection_plan)
        .expect("apply projection");

    let derive_plan = plan_derivation(&app).expect("derive");
    assert_eq!(derive_plan.episodes.len(), 2);
    app.apply_derivation(&derive_plan).expect("apply derive");
    let search_doc_count: i64 = conn
        .query_row("select count(*) from search_doc_redacted", [], |row| {
            row.get(0)
        })
        .expect("search doc count");
    assert_eq!(search_doc_count, 2);

    let results = app
        .search_episodes(SearchEpisodesRequest {
            query: "timeout error".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("search");
    assert!(!results.is_empty());
    assert_eq!(results[0].problem, "Timeout error in worker");

    let evidence_results = app
        .search_episodes(SearchEpisodesRequest {
            query: "exit code: 0".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("evidence search");
    assert!(
        !evidence_results.is_empty(),
        "raw evidence fallback should surface tool-result text"
    );

    let runbook = app.get_runbook(&results[0].episode_id).expect("runbook");
    assert_eq!(runbook.commands, vec!["cargo test -p axiomsync"]);
    assert_eq!(runbook.verification[0].status, VerificationStatus::Pass);

    let replan = plan_ingest(&app, &sample_batch("codex")).expect("replan");
    assert_eq!(replan.adds.len(), 0);
}

#[test]
fn sink_contract_parity_holds_for_all_connectors() {
    for connector in ["chatgpt", "codex", "claude_code", "gemini_cli"] {
        let app = mock_app();
        let batch = sample_batch(connector);
        let append_request = AppendRawEventsRequest {
            request_id: Some(format!("req-{connector}")),
            events: batch.events.clone(),
        };
        let first_batch = app
            .build_append_batch(&append_request)
            .expect("append batch first");
        let first_existing = app.load_existing_raw_event_keys().expect("existing first");
        let first_plan = app
            .plan_ingest(&first_existing, &first_batch)
            .expect("sink append first");
        assert_eq!(first_plan.adds.len(), 4, "{connector}");
        assert_eq!(first_plan.skipped_dedupe_keys.len(), 0, "{connector}");
        app.apply_ingest(&first_plan).expect("apply first");

        let second_batch = app
            .build_append_batch(&append_request)
            .expect("append batch second");
        let second_existing = app.load_existing_raw_event_keys().expect("existing second");
        let second_plan = app
            .plan_ingest(&second_existing, &second_batch)
            .expect("sink append second");
        assert_eq!(second_plan.adds.len(), 0, "{connector}");
        assert_eq!(second_plan.skipped_dedupe_keys.len(), 4, "{connector}");

        let cursor_request = UpsertSourceCursorRequest {
            source: connector.to_string(),
            cursor: batch.cursor.clone().expect("cursor"),
        };
        let cursor_plan = app
            .plan_source_cursor_upsert(&cursor_request)
            .expect("cursor plan");
        let cursor_response = app
            .apply_source_cursor_upsert(&cursor_plan)
            .expect("cursor upsert");
        assert_eq!(
            cursor_response["cursor"]["connector"].as_str(),
            Some(connector),
            "{connector}"
        );

        let conn = Connection::open(app.db_path()).expect("sqlite");
        let raw_count: i64 = conn
            .query_row("select count(*) from raw_event", [], |row| row.get(0))
            .expect("count");
        let cursor_count: i64 = conn
            .query_row("select count(*) from source_cursor", [], |row| row.get(0))
            .expect("count");
        assert_eq!(raw_count, 4, "{connector}");
        assert_eq!(cursor_count, 1, "{connector}");
    }
}

#[test]
fn apply_projection_rejects_turn_without_item() {
    let app = mock_app();
    app.init().expect("init");
    let invalid = ProjectionPlan {
        workspaces: vec![WorkspaceRow {
            stable_id: "ws_invalid".to_string(),
            canonical_root: "/repo/app".to_string(),
            repo_remote: None,
            branch: None,
            worktree_path: None,
        }],
        conv_sessions: vec![ConvSessionRow {
            stable_id: "session_invalid".to_string(),
            connector: "codex".to_string(),
            native_session_id: "native-session".to_string(),
            workspace_id: Some("ws_invalid".to_string()),
            title: None,
            transcript_uri: None,
            status: "active".to_string(),
            started_at_ms: Some(1),
            ended_at_ms: Some(2),
        }],
        conv_turns: vec![ConvTurnRow {
            stable_id: "turn_invalid".to_string(),
            session_id: "session_invalid".to_string(),
            native_turn_id: Some("turn-1".to_string()),
            turn_index: 0,
            actor: "user".to_string(),
        }],
        conv_items: vec![],
        artifacts: vec![],
        evidence_anchors: vec![],
        execution_runs: vec![],
        execution_tasks: vec![],
        execution_checks: vec![],
        execution_approvals: vec![],
        execution_events: vec![],
        document_records: vec![],
    };
    assert!(app.apply_projection(&invalid).is_err());
}

#[test]
fn universal_agent_records_project_to_execution_and_document_views_only() {
    let app = mock_app();
    let batch = ConnectorBatchInput {
        events: vec![
            RawEventInput {
                source: "codex".to_string(),
                native_schema_version: Some("v1".to_string()),
                native_session_id: "thread-session".to_string(),
                native_event_id: Some("thread-1".to_string()),
                event_type: "user_message".to_string(),
                ts_ms: 1_710_000_000_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "turn_id": "turn-1",
                    "actor": "user",
                    "text": "Investigate timeout error"
                }),
            },
            RawEventInput {
                source: "codex".to_string(),
                native_schema_version: Some("agent-record-v1".to_string()),
                native_session_id: "runtime-session".to_string(),
                native_event_id: Some("run-1".to_string()),
                event_type: "task_state".to_string(),
                ts_ms: 1_710_000_001_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "record_type": "task_state",
                    "subject": {"kind": "task", "id": "task-1", "parent_id": "run-1"},
                    "runtime": {"run_id": "run-1", "task_id": "task-1", "role": "do", "status": "running"},
                    "task": {"title": "Investigate timeout"},
                    "body": {"text": "Task is running"}
                }),
            },
            RawEventInput {
                source: "codex".to_string(),
                native_schema_version: Some("agent-record-v1".to_string()),
                native_session_id: "runtime-session".to_string(),
                native_event_id: Some("doc-1".to_string()),
                event_type: "document_snapshot".to_string(),
                ts_ms: 1_710_000_002_000,
                payload: json!({
                    "workspace_root": "/repo/app",
                    "record_type": "document_snapshot",
                    "subject": {"kind": "document", "id": "mission-doc"},
                    "document": {
                        "kind": "mission",
                        "path": "program/MISSION.md",
                        "title": "Mission",
                        "body": "Restore service health"
                    },
                    "artifacts": [
                        {
                            "uri": "file:///repo/app/program/MISSION.md",
                            "mime": "text/markdown",
                            "sha256": "abc123",
                            "bytes": 23
                        }
                    ]
                }),
            },
        ],
        cursor: None,
    };

    app.init().expect("init");
    let ingest = plan_ingest(&app, &batch).expect("plan ingest");
    app.apply_ingest(&ingest).expect("apply ingest");

    let raw_events = app.load_raw_events().expect("raw events");
    let projection = app.plan_projection(&raw_events).expect("projection");
    assert_eq!(projection.conv_sessions.len(), 1);
    assert_eq!(projection.execution_runs.len(), 1);
    assert_eq!(projection.execution_tasks.len(), 1);
    assert_eq!(projection.execution_events.len(), 1);
    assert_eq!(projection.document_records.len(), 1);
    assert_eq!(projection.conv_items.len(), 1);
    app.apply_projection(&projection).expect("apply projection");

    let runs = app.list_runs(None).expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, "run-1");
    assert_eq!(runs[0].producer, "codex");

    let documents = app
        .list_documents(None, Some("mission"))
        .expect("documents");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].document_id, "mission-doc");
    assert_eq!(documents[0].kind, "mission");

    let derive = plan_derivation(&app).expect("derive");
    assert_eq!(derive.episodes.len(), 1);
}

#[test]
fn apply_derivation_rejects_episode_without_member() {
    let app = mock_app();
    app.init().expect("init");
    let invalid = DerivePlan {
        episodes: vec![EpisodeRow {
            stable_id: "episode_invalid".to_string(),
            workspace_id: None,
            problem_signature: "sig".to_string(),
            status: EpisodeStatus::Open,
            opened_at_ms: 1,
            closed_at_ms: None,
        }],
        episode_members: vec![],
        insights: vec![],
        insight_anchors: vec![],
        verifications: vec![],
        search_docs_redacted: vec![SearchDocRedactedRow {
            stable_id: "doc_invalid".to_string(),
            episode_id: "episode_invalid".to_string(),
            body: "problem: timeout".to_string(),
        }],
    };
    assert!(app.apply_derivation(&invalid).is_err());
}

#[test]
fn replay_doctor_purge_and_repair_stay_deterministic() {
    let app = mock_app();
    let batch = sample_batch("codex");

    let ingest = plan_ingest(&app, &batch).expect("plan ingest");
    app.apply_ingest(&ingest).expect("apply ingest");

    let replay = plan_replay(&app).expect("plan replay");
    app.apply_replay(&replay).expect("apply replay");
    let baseline_runbooks = app.list_runbooks().expect("runbooks");
    let baseline_results = app
        .search_episodes(SearchEpisodesRequest {
            query: "timeout".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("search");

    let second_replay = plan_replay(&app).expect("plan replay second");
    assert_eq!(replay, second_replay);
    app.apply_replay(&second_replay)
        .expect("apply replay second");
    assert_eq!(
        baseline_runbooks,
        app.list_runbooks().expect("runbooks second")
    );
    assert_eq!(
        baseline_results,
        app.search_episodes(SearchEpisodesRequest {
            query: "timeout".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("search second")
    );

    let doctor = app.doctor().expect("doctor");
    assert!(!doctor.drift_detected);
    assert_eq!(
        doctor.stored_schema_version.as_deref(),
        Some(axiomsync::domain::RENEWAL_SCHEMA_VERSION)
    );

    let purge = plan_purge(&app, Some("codex"), None).expect("plan purge");
    assert_eq!(purge.deleted_raw_event_ids.len(), 4);
    app.apply_purge(&purge).expect("apply purge");

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let raw_count: i64 = conn
        .query_row("select count(*) from raw_event", [], |row| row.get(0))
        .expect("raw count");
    let episode_count: i64 = conn
        .query_row("select count(*) from episode", [], |row| row.get(0))
        .expect("episode count");
    assert_eq!(raw_count, 0);
    assert_eq!(episode_count, 0);

    let repair = plan_repair(&app, &batch).expect("plan repair");
    app.apply_repair(&repair).expect("apply repair");
    assert_eq!(
        baseline_runbooks,
        app.list_runbooks().expect("runbooks repaired")
    );
    assert_eq!(
        baseline_results,
        app.search_episodes(SearchEpisodesRequest {
            query: "timeout".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("search repaired")
    );
}

#[test]
fn doctor_detects_schema_version_drift() {
    let app = mock_app();
    app.init().expect("init");
    let conn = Connection::open(app.db_path()).expect("sqlite");
    conn.execute(
        "update axiomsync_meta set value = 'stale-version' where key = 'schema_version'",
        [],
    )
    .expect("update version");

    let doctor = app.doctor().expect("doctor");
    assert!(doctor.drift_detected);
    assert!(doctor.version_mismatch);
    assert_eq!(
        doctor.stored_schema_version.as_deref(),
        Some("stale-version")
    );
}
