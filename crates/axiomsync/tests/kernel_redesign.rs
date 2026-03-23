use std::fs;

use axiomsync::domain::{
    AppendRawEventsRequest, SearchClaimsRequest, SearchEntriesRequest, SearchFilter,
    SearchProceduresRequest, UpsertSourceCursorRequest, workspace_stable_id,
};
use rusqlite::Connection;
use tempfile::tempdir;

fn sample_request() -> AppendRawEventsRequest {
    serde_json::from_value(serde_json::json!({
        "request_id": "req-1",
        "events": [
            {
                "connector": "relay",
                "native_session_id": "session-1",
                "native_event_id": "evt-1",
                "event_type": "assistant_message",
                "ts_ms": 1710000000000i64,
                "workspace_root": "/workspace/demo",
                "payload": {
                    "title": "Session title",
                    "text": "Root cause: bad cursor handling\nFix: reset and rebuild\nDecision: keep sink narrow\n$ cargo test -q"
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
                "source": "relay",
                "source_kind": "relay",
                "session_kind": "run",
                "external_session_key": "run-1",
                "external_entry_key": "run-evt-1",
                "event_kind": "run_update",
                "observed_at": "2024-03-09T16:00:01Z",
                "workspace_root": "/workspace/demo",
                "payload": {
                    "title": "Run 1",
                    "text": "Run completed successfully\n$ cargo test -q"
                }
            },
            {
                "source": "relay",
                "session_kind": "task",
                "external_session_key": "task-1",
                "external_entry_key": "task-evt-1",
                "event_kind": "task_update",
                "observed_at": "2024-03-09T16:00:02Z",
                "workspace_root": "/workspace/demo",
                "payload": {
                    "title": "Task 1",
                    "text": "Task started"
                }
            },
            {
                "source": "relay",
                "session_kind": "task",
                "external_session_key": "task-1",
                "external_entry_key": "task-check-1",
                "event_kind": "check_result",
                "observed_at": "2024-03-09T16:00:03Z",
                "workspace_root": "/workspace/demo",
                "payload": {
                    "text": "Check passed for task 1"
                }
            },
            {
                "source": "relay",
                "session_kind": "import",
                "external_session_key": "import-1",
                "external_entry_key": "doc-1",
                "event_kind": "document_snapshot",
                "observed_at": "2024-03-09T16:00:04Z",
                "workspace_root": "/workspace/demo",
                "payload": {
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
    let app = axiomsync::open(temp.path()).expect("app");

    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    app.rebuild().expect("rebuild");

    let sessions = app.list_sessions().expect("sessions");
    assert_eq!(sessions.len(), 4);
    assert!(
        sessions
            .iter()
            .any(|session| session.session_kind == "conversation")
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

    let conversation_id = sessions
        .iter()
        .find(|session| session.session_kind == "conversation")
        .expect("conversation")
        .session_id
        .clone();
    let task_id = sessions
        .iter()
        .find(|session| session.session_kind == "task")
        .expect("task")
        .session_id
        .clone();
    let import_id = sessions
        .iter()
        .find(|session| session.session_kind == "import")
        .expect("import")
        .session_id
        .clone();

    let session = app.get_session(&conversation_id).expect("session");
    assert_eq!(session.entries.len(), 1);
    assert_eq!(session.entries[0].artifacts.len(), 1);
    assert_eq!(session.entries[0].anchors.len(), 2);

    let task = app.get_task(&task_id).expect("task view");
    assert_eq!(task.session.session_kind, "task");
    assert_eq!(task.entries.len(), 2);

    let imported = app.get_session(&import_id).expect("import session");
    assert_eq!(imported.session.session_kind, "import");
    assert_eq!(imported.entries.len(), 1);
    assert_eq!(imported.entries[0].artifacts.len(), 1);

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

    let claims_before = app
        .search_claims(SearchClaimsRequest {
            query: "root cause".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("search claims");
    assert!(!claims_before.is_empty());

    let procedures_before = app
        .search_procedures(SearchProceduresRequest {
            query: "cargo test".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("search procedures");
    assert!(!procedures_before.is_empty());

    app.rebuild().expect("rebuild again");

    let claims_after = app
        .search_claims(SearchClaimsRequest {
            query: "root cause".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("search claims after rebuild");
    let procedures_after = app
        .search_procedures(SearchProceduresRequest {
            query: "cargo test".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("search procedures after rebuild");
    assert_eq!(claims_before, claims_after);
    assert_eq!(procedures_before, procedures_after);

    let case = app.get_case(&episode_id).expect("case");
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
    let app = axiomsync::open(temp.path()).expect("app");
    let ingest = app
        .plan_append_raw_events(sample_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    app.rebuild().expect("rebuild");

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

    let hits = app
        .search_entries(SearchEntriesRequest {
            query: "cursor".to_string(),
            limit: 10,
            filter: SearchFilter {
                session_kind: Some("conversation".to_string()),
                connector: Some("relay".to_string()),
                workspace_root: Some("/workspace/demo".to_string()),
            },
        })
        .expect("search entries");
    assert_eq!(hits.len(), 1);
}

#[test]
fn duplicate_dedupe_and_source_cursor_upsert_are_idempotent() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");

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
        "cursor": {
            "cursor_key": "chat",
            "cursor_value": "cursor-1",
            "updated_at_ms": 1710000005000i64
        }
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
fn schema_contains_only_new_projection_tables() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");
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
fn legacy_raw_event_table_is_backfilled_into_ingress_receipts() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("context.db");
    let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
    conn.execute_batch(
        r#"
        create table raw_event (
          id integer primary key autoincrement,
          stable_id text not null default '',
          connector text not null,
          native_schema_version text,
          native_session_id text not null,
          native_event_id text,
          event_type text not null,
          ts_ms integer not null,
          payload_json text not null,
          payload_sha256 blob not null
        );
        insert into raw_event (
          stable_id, connector, native_schema_version, native_session_id, native_event_id,
          event_type, ts_ms, payload_json, payload_sha256
        ) values (
          'raw_1', 'relay', 'agent-record-v1', 'legacy-session', 'evt-1',
          'assistant_message', 1710000000000,
          '{"text":"legacy body"}',
          x'0123'
        );
        "#,
    )
    .expect("seed legacy raw_event");
    drop(conn);

    let app = axiomsync::open(temp.path()).expect("app");
    app.rebuild().expect("rebuild after migration");
    let report = app.doctor_report().expect("doctor");
    assert_eq!(report.ingress_receipts, 1);
    assert!(fs::metadata(temp.path().join("context.db")).is_ok());
}

#[test]
fn derivation_plan_materializes_search_docs_before_apply() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");
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
