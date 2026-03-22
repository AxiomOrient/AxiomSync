use std::sync::Arc;

use axiomsync::domain::{
    ConnectorBatchInput, CursorInput, EpisodeExtraction, RawEventInput, SearchEpisodesFilter,
    SearchEpisodesRequest, VerificationExtraction, VerificationKind, VerificationStatus,
};
use axiomsync::kernel::AxiomSync;
use axiomsync::llm::MockLlmClient;
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

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
                connector: connector.to_string(),
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
                connector: connector.to_string(),
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
                connector: connector.to_string(),
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
                connector: connector.to_string(),
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
    app.init().expect("init");
    assert!(app.db_path().exists());

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

    let ingest_plan = app
        .plan_ingest(&sample_batch("codex"))
        .expect("plan ingest");
    assert_eq!(ingest_plan.adds.len(), 4);
    app.apply_ingest(&ingest_plan).expect("apply ingest");
    let conn = Connection::open(app.db_path()).expect("sqlite");
    let journal_count: i64 = conn
        .query_row("select count(*) from import_journal", [], |row| row.get(0))
        .expect("journal count");
    assert_eq!(journal_count, 1);

    let projection_plan = app.plan_projection().expect("projection");
    assert_eq!(projection_plan.conv_sessions.len(), 1);
    app.apply_projection(&projection_plan)
        .expect("apply projection");

    let derive_plan = app.plan_derivation().expect("derive");
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

    let replan = app.plan_ingest(&sample_batch("codex")).expect("replan");
    assert_eq!(replan.adds.len(), 0);
}

#[test]
fn connector_dedup_and_cursor_work_for_all_connectors() {
    for connector in ["chatgpt", "codex", "claude_code", "gemini_cli"] {
        let app = mock_app();
        let plan = app.plan_ingest(&sample_batch(connector)).expect("plan");
        app.apply_ingest(&plan).expect("apply");
        let second = app
            .plan_ingest(&sample_batch(connector))
            .expect("plan second");
        assert_eq!(second.adds.len(), 0, "{connector}");
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
fn replay_doctor_purge_and_repair_stay_deterministic() {
    let app = mock_app();
    let batch = sample_batch("codex");

    let ingest = app.plan_ingest(&batch).expect("plan ingest");
    app.apply_ingest(&ingest).expect("apply ingest");

    let replay = app.plan_replay().expect("plan replay");
    app.apply_replay(&replay).expect("apply replay");
    let baseline_runbooks = app.list_runbooks().expect("runbooks");
    let baseline_results = app
        .search_episodes(SearchEpisodesRequest {
            query: "timeout".to_string(),
            limit: 10,
            filter: SearchEpisodesFilter::default(),
        })
        .expect("search");

    let second_replay = app.plan_replay().expect("plan replay second");
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

    let purge = app.plan_purge(Some("codex"), None).expect("plan purge");
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

    let repair = app.plan_repair(&batch).expect("plan repair");
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
