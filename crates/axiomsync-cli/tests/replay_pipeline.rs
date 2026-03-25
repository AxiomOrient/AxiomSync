use axiomsync_domain::{
    AppendRawEventsRequest, SearchCasesRequest, SearchFilter, SessionRow,
    UpsertSourceCursorRequest, workspace_stable_id,
};
use axiomsync_kernel::AxiomSync;
use rusqlite::Connection;
use tempfile::tempdir;

fn apply_replay_plan(app: &AxiomSync) {
    let plan = app.build_replay_plan().expect("replay plan");
    app.apply_replay(&plan).expect("apply replay plan");
}

fn sample_request() -> AppendRawEventsRequest {
    serde_json::from_value(serde_json::json!({
        "batch_id": "req-1",
        "producer": "relay",
        "received_at_ms": 1710000004000i64,
        "events": [
            {
                "connector": "chatgpt_web_selection",
                "native_session_id": "session-1",
                "native_event_id": "evt-1",
                "event_type": "selection_captured",
                "ts_ms": 1710000000000i64,
                "payload": {
                    "session_kind": "thread",
                    "workspace_root": "/workspace/demo",
                    "page_title": "Session title",
                    "selection": {
                        "text": "Root cause: bad cursor handling\nFix: reset and rebuild\nDecision: keep sink narrow\n$ cargo test -q",
                        "start_hint": "Root cause: bad cursor handling",
                        "end_hint": "$ cargo test -q",
                        "dom_fingerprint": "sha1:demo:selection"
                    },
                    "source_message": {
                        "message_id": "msg-1",
                        "role": "assistant"
                    }
                },
                "artifacts": [
                    {
                        "artifact_kind": "file",
                        "uri": "file:///workspace/demo/src/lib.rs",
                        "mime_type": "text/rust"
                    }
                ]
            },
            {
                "connector": "cli_local_exec",
                "native_session_id": "run-1",
                "native_event_id": "run-evt-1",
                "event_type": "command_finished",
                "ts_ms": 1710000001000i64,
                "payload": {
                    "session_kind": "run",
                    "workspace_root": "/workspace/demo",
                    "title": "Run 1",
                    "summary": "Run completed successfully\n$ cargo test -q",
                    "command": {
                        "argv": ["cargo", "test", "-q"],
                        "cwd": "/workspace/demo",
                        "exit_code": 0,
                        "duration_ms": 500
                    },
                    "checks": [
                        {
                            "name": "cargo_test",
                            "status": "passed"
                        }
                    ]
                }
            },
            {
                "connector": "work_state_export",
                "native_session_id": "task-1",
                "native_event_id": "task-evt-1",
                "event_type": "task_state_imported",
                "ts_ms": 1710000002000i64,
                "payload": {
                    "session_kind": "task",
                    "workspace_root": "/workspace/demo",
                    "title": "Task 1",
                    "text": "Task started"
                }
            },
            {
                "connector": "work_state_export",
                "native_session_id": "task-1",
                "native_event_id": "task-check-1",
                "event_type": "verification_recorded",
                "ts_ms": 1710000003000i64,
                "payload": {
                    "session_kind": "task",
                    "workspace_root": "/workspace/demo",
                    "text": "Check passed for task 1"
                },
                "hints": {
                    "entry_kind": "check_result"
                }
            },
            {
                "connector": "work_state_export",
                "native_session_id": "import-1",
                "native_event_id": "doc-1",
                "event_type": "artifact_emitted",
                "ts_ms": 1710000004000i64,
                "payload": {
                    "session_kind": "import",
                    "workspace_root": "/workspace/demo",
                    "title": "Imported document",
                    "text": "Imported notes for rebuild"
                },
                "artifacts": [
                    {
                        "artifact_kind": "document",
                        "uri": "file:///workspace/demo/docs/notes.md",
                        "mime_type": "text/markdown"
                    }
                ]
            }
        ]
    }))
    .expect("request")
}

#[test]
fn redesign_pipeline_projects_and_derives_generic_kernel_rows() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let sessions = app.list_sessions().expect("sessions");
    assert_eq!(sessions.len(), 4);
    assert!(
        sessions
            .iter()
            .any(|session| session.session_kind == "thread")
    );
    assert!(sessions.iter().any(|session| session.session_kind == "run"));
    assert!(
        sessions
            .iter()
            .any(|session| session.session_kind == "task")
    );
    assert!(
        sessions
            .iter()
            .any(|session| session.session_kind == "import")
    );

    let thread_id = sessions
        .iter()
        .find(|session| session.session_kind == "thread")
        .expect("thread")
        .session_id
        .clone();
    let task_id = sessions
        .iter()
        .find(|session| session.session_kind == "task")
        .expect("task")
        .session_id
        .clone();
    let thread = app.get_thread(&thread_id).expect("thread");
    assert_eq!(thread.entries.len(), 1);
    assert_eq!(thread.entries[0].artifacts.len(), 1);
    assert_eq!(thread.entries[0].anchors.len(), 2);

    let task = app.get_task(&task_id).expect("task view");
    assert_eq!(task.session.session_kind, "task");
    assert_eq!(task.entries.len(), 2);

    let imported_documents = app
        .list_documents(Some("/workspace/demo"), Some("document"))
        .expect("imported documents");
    assert_eq!(imported_documents.len(), 1);
    assert!(
        imported_documents[0]
            .artifact
            .uri
            .ends_with("/workspace/demo/docs/notes.md")
    );

    let episode_id = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| {
            case.commands
                .iter()
                .any(|command| command.contains("cargo test -q"))
        })
        .expect("case with command")
        .case_id;

    let case_before = app.get_case(&episode_id).expect("case before replay");

    apply_replay_plan(&app);

    let case = app.get_case(&episode_id).expect("case after replay");
    assert_eq!(case_before, case);
    assert_eq!(case.workspace_root.as_deref(), Some("/workspace/demo"));
    assert!(
        case.commands
            .iter()
            .any(|command| command.contains("cargo test -q"))
    );
}

#[test]
fn auth_grants_and_search_filters_are_workspace_scoped() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let plan = app
        .plan_workspace_token_grant("/workspace/demo", "workspace-token")
        .expect("grant plan");
    app.apply_workspace_token_grant(&plan).expect("apply grant");

    let workspace_id = workspace_stable_id("/workspace/demo");
    assert_eq!(
        app.authorize_workspace("workspace-token", Some(&workspace_id))
            .expect("authorize"),
        Some(workspace_id)
    );
    assert!(app.authorize_workspace("workspace-token", None).is_err());

    let case_query = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| {
            case.workspace_root.as_deref() == Some("/workspace/demo") && case.root_cause.is_some()
        })
        .expect("thread-backed workspace case")
        .problem;
    let hits = app
        .search_cases(SearchCasesRequest {
            query: case_query,
            limit: 10,
            filter: SearchFilter {
                session_kind: Some("thread".to_string()),
                connector: Some("chatgpt_web_selection".to_string()),
                workspace_root: Some("/workspace/demo".to_string()),
            },
        })
        .expect("search cases");
    assert_eq!(hits.len(), 1);
}

#[test]
fn case_commands_follow_stable_episode_relation_not_goal_text() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let mut plan = app.build_derivation_plan().expect("derivation plan");
    let procedure = plan
        .procedures
        .iter_mut()
        .find(|procedure| {
            procedure
                .steps_json
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|step| step.as_str())
                .any(|step| step.contains("cargo test -q"))
        })
        .expect("procedure with command");
    procedure.goal = Some("mismatched free-text goal".to_string());

    app.apply_derivation_plan(&plan).expect("apply derivation");

    let case = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| case.workspace_root.as_deref() == Some("/workspace/demo"))
        .expect("workspace case");
    assert!(
        case.commands
            .iter()
            .any(|command| command.contains("cargo test -q"))
    );
}

#[test]
fn duplicate_dedupe_and_source_cursor_upsert_are_idempotent() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    let first_plan = app
        .plan_append_raw_events(sample_request())
        .expect("first ingest plan");
    assert_eq!(first_plan.skipped_dedupe_keys.len(), 0);
    app.apply_ingest_plan(&first_plan)
        .expect("apply first ingest");

    let second_plan = app
        .plan_append_raw_events(sample_request())
        .expect("second ingest plan");
    assert_eq!(second_plan.receipts.len(), 0);
    assert_eq!(second_plan.skipped_dedupe_keys.len(), 5);
    app.apply_ingest_plan(&second_plan)
        .expect("apply second ingest");

    let cursor_request: UpsertSourceCursorRequest = serde_json::from_value(serde_json::json!({
        "connector": "relay",
        "cursor_key": "chat",
        "cursor_value": "cursor-1",
        "updated_at_ms": 1710000005000i64
    }))
    .expect("cursor request");
    let cursor_plan = app
        .plan_source_cursor_upsert(cursor_request)
        .expect("cursor plan");
    app.apply_source_cursor_plan(&cursor_plan)
        .expect("apply cursor once");
    app.apply_source_cursor_plan(&cursor_plan)
        .expect("apply cursor twice");

    let conn = Connection::open(app.db_path()).expect("open sqlite");
    let receipt_count: i64 = conn
        .query_row("select count(*) from ingress_receipts", [], |row| {
            row.get(0)
        })
        .expect("receipt count");
    assert_eq!(receipt_count, 5);

    let cursor_count: i64 = conn
        .query_row("select count(*) from source_cursor", [], |row| row.get(0))
        .expect("cursor count");
    assert_eq!(cursor_count, 1);
    let cursor_value: String = conn
        .query_row(
            "select cursor_value from source_cursor where connector = 'relay' and cursor_key = 'chat'",
            [],
            |row| row.get(0),
        )
        .expect("cursor value");
    assert_eq!(cursor_value, "cursor-1");
    let cursor_metadata: String = conn
        .query_row(
            "select metadata_json from source_cursor where connector = 'relay' and cursor_key = 'chat'",
            [],
            |row| row.get(0),
        )
        .expect("cursor metadata");
    assert_eq!(cursor_metadata, "{}");
}

#[test]
fn ingest_plan_dedupes_duplicates_within_same_batch() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    let mut request = serde_json::to_value(sample_request()).expect("request json");
    let duplicate_event = request["events"][0].clone();
    request["events"]
        .as_array_mut()
        .expect("events array")
        .push(duplicate_event);

    let plan = app
        .plan_append_raw_events(serde_json::from_value(request).expect("request"))
        .expect("ingest plan");
    assert_eq!(plan.receipts.len(), 5);
    assert_eq!(plan.skipped_dedupe_keys.len(), 1);

    app.apply_ingest_plan(&plan).expect("apply ingest");

    let conn = Connection::open(app.db_path()).expect("open sqlite");
    let receipt_count: i64 = conn
        .query_row("select count(*) from ingress_receipts", [], |row| {
            row.get(0)
        })
        .expect("receipt count");
    assert_eq!(receipt_count, 5);
}

#[test]
fn replay_apply_is_atomic_when_derivation_write_fails() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let conn = Connection::open(app.db_path()).expect("open sqlite");
    let session_count_before: i64 = conn
        .query_row("select count(*) from sessions", [], |row| row.get(0))
        .expect("session count before");
    drop(conn);

    let mut plan = app.build_replay_plan().expect("replay plan");
    plan.projection.sessions.push(SessionRow {
        session_id: "session_extra".to_string(),
        session_kind: "thread".to_string(),
        connector: "relay".to_string(),
        external_session_key: Some("extra".to_string()),
        title: Some("extra".to_string()),
        workspace_root: Some("/workspace/demo".to_string()),
        opened_at: None,
        closed_at: None,
        metadata_json: serde_json::json!({}),
    });
    plan.derivation.episodes[0].session_id = Some("missing-session".to_string());

    assert!(app.apply_replay(&plan).is_err());

    let conn = Connection::open(app.db_path()).expect("open sqlite");
    let session_count: i64 = conn
        .query_row("select count(*) from sessions", [], |row| row.get(0))
        .expect("session count");
    assert_eq!(session_count, session_count_before);
    let extra_session_count: i64 = conn
        .query_row(
            "select count(*) from sessions where session_id = 'session_extra'",
            [],
            |row| row.get(0),
        )
        .expect("extra session count");
    assert_eq!(extra_session_count, 0);
}

#[test]
fn schema_contains_only_new_projection_tables() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let conn = Connection::open(app.db_path()).expect("open sqlite");

    for table in [
        "ingress_receipts",
        "source_cursor",
        "sessions",
        "entries",
        "artifacts",
        "anchors",
        "episodes",
        "insights",
        "verifications",
        "claims",
        "procedures",
        "search_docs",
    ] {
        let exists: i64 = conn
            .query_row(
                "select count(*) from sqlite_master where type = 'table' and name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table check");
        assert_eq!(exists, 1, "expected table {table}");
    }

    for table in [
        "conv_session",
        "conv_turn",
        "conv_item",
        "execution_run",
        "execution_task",
        "document_record",
        "search_doc_redacted",
    ] {
        let exists: i64 = conn
            .query_row(
                "select count(*) from sqlite_master where type = 'table' and name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("legacy table check");
        assert_eq!(exists, 0, "did not expect table {table}");
    }
}

#[test]
fn derivation_plan_materializes_search_docs_before_apply() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");

    let projection = app.build_projection_plan().expect("projection plan");
    app.apply_projection_plan(&projection)
        .expect("apply projection plan");

    let derivation = app.build_derivation_plan().expect("derivation plan");
    assert!(!derivation.search_docs.is_empty());
    assert!(
        derivation
            .search_docs
            .iter()
            .any(|doc| doc.subject_kind == "insight")
    );
}

#[test]
fn doctor_report_tracks_pending_state_lifecycle() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");

    let report_after_ingest = app.doctor_report().expect("doctor after ingest");
    assert_eq!(report_after_ingest.pending_projection_count, 5);
    assert_eq!(report_after_ingest.pending_derived_count, 5);
    assert_eq!(report_after_ingest.pending_index_count, 5);

    let projection = app.build_projection_plan().expect("projection");
    app.apply_projection_plan(&projection)
        .expect("apply projection");
    let report_after_projection = app.doctor_report().expect("doctor after projection");
    assert_eq!(report_after_projection.pending_projection_count, 0);
    assert_eq!(report_after_projection.pending_derived_count, 5);
    assert_eq!(report_after_projection.pending_index_count, 5);

    let derivation = app.build_derivation_plan().expect("derivation");
    app.apply_derivation_plan(&derivation)
        .expect("apply derivation");
    let report_after_derivation = app.doctor_report().expect("doctor after derivation");
    assert_eq!(report_after_derivation.pending_projection_count, 0);
    assert_eq!(report_after_derivation.pending_derived_count, 0);
    assert_eq!(report_after_derivation.pending_index_count, 0);
}

#[test]
fn context_db_sets_current_user_version() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let conn = Connection::open(app.db_path()).expect("open sqlite");
    let user_version: i64 = conn
        .query_row("pragma user_version", [], |row| row.get(0))
        .expect("user_version");
    assert_eq!(user_version, 2);
}

#[test]
fn search_ranking_prefers_verified_cases_with_more_evidence() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let request: AppendRawEventsRequest = serde_json::from_value(serde_json::json!({
        "batch_id": "ranking-batch",
        "producer": "relay",
        "received_at_ms": 1710000010000i64,
        "events": [
            {
                "connector": "chatgpt_web_selection",
                "native_session_id": "thread-verified",
                "native_event_id": "evt-verified-1",
                "event_type": "selection_captured",
                "ts_ms": 1710000010000i64,
                "payload": {
                    "session_kind": "thread",
                    "workspace_root": "/workspace/ranking",
                    "page_title": "Verified queue drift",
                    "selection": {
                        "text": "Root cause: queue drift\nFix: rebuild projection\nDecision: keep sink narrow",
                        "start_hint": "Root cause: queue drift",
                        "end_hint": "Decision: keep sink narrow",
                        "dom_fingerprint": "sha1:ranking:verified"
                    }
                }
            },
            {
                "connector": "chatgpt_web_selection",
                "native_session_id": "thread-verified",
                "native_event_id": "evt-verified-2",
                "event_type": "verification_recorded",
                "ts_ms": 1710000010100i64,
                "payload": {
                    "session_kind": "thread",
                    "workspace_root": "/workspace/ranking",
                    "text": "Check passed for queue drift fix"
                },
                "hints": {
                    "entry_kind": "check_result"
                }
            },
            {
                "connector": "chatgpt_web_selection",
                "native_session_id": "thread-proposed",
                "native_event_id": "evt-proposed-1",
                "event_type": "selection_captured",
                "ts_ms": 1710000010200i64,
                "payload": {
                    "session_kind": "thread",
                    "workspace_root": "/workspace/ranking",
                    "page_title": "Proposed queue drift",
                    "selection": {
                        "text": "Root cause: queue drift",
                        "start_hint": "Root cause: queue drift",
                        "end_hint": "Root cause: queue drift",
                        "dom_fingerprint": "sha1:ranking:proposed"
                    }
                }
            }
        ]
    }))
    .expect("request");

    let ingest = app.plan_append_raw_events(request).expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let verified_case_id = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| {
            case.workspace_root.as_deref() == Some("/workspace/ranking")
                && !case.verification.is_empty()
        })
        .expect("verified case")
        .case_id;

    let hits = app
        .search_cases(SearchCasesRequest {
            query: "queue drift".to_string(),
            limit: 10,
            filter: SearchFilter {
                session_kind: Some("thread".to_string()),
                connector: Some("chatgpt_web_selection".to_string()),
                workspace_root: Some("/workspace/ranking".to_string()),
            },
        })
        .expect("search cases");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].id, verified_case_id);
    assert!(!hits[0].evidence.is_empty());
    assert!(hits[0].score > hits[1].score);
}

#[cfg(unix)]
#[test]
fn auth_store_is_written_with_owner_only_permissions() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let plan = app
        .plan_workspace_token_grant("/workspace/demo", "workspace-token")
        .expect("grant plan");
    app.apply_workspace_token_grant(&plan).expect("apply grant");

    let metadata = fs::metadata(app.auth_path()).expect("auth metadata");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}
