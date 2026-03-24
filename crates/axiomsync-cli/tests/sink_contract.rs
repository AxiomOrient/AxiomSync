use std::fs;
use std::path::Path;

use axiomsync_domain::{AppendRawEventsRequest, RAW_EVENT_TAXONOMY};
use axiomsync_kernel::AxiomSync;
use jsonschema::validator_for;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

fn apply_replay_plan(app: &AxiomSync) {
    let plan = app.build_replay_plan().expect("replay plan");
    app.apply_replay(&plan).expect("apply replay plan");
}

fn fixture_request(name: &str) -> AppendRawEventsRequest {
    serde_json::from_value(fixture_value(name)).expect("fixture request")
}

fn fixture_value(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    serde_json::from_slice(&fs::read(path).expect("fixture file")).expect("fixture value")
}

fn sink_schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/kernel_sink_contract.json");
    serde_json::from_slice(&fs::read(path).expect("schema file")).expect("schema value")
}

#[test]
fn final_form_examples_are_accepted_projected_and_derived() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");

    for name in [
        "raw_event.chatgpt_selection.json",
        "raw_event.axiomrams_run_summary.json",
    ] {
        let request = fixture_request(name);
        let plan = app.plan_append_raw_events(request).expect("plan ingest");
        assert_eq!(plan.receipts.len(), 1, "expected one receipt for {name}");
        app.apply_ingest_plan(&plan).expect("apply ingest");
    }

    let report_before = app.doctor_report().expect("doctor before rebuild");
    assert_eq!(report_before.ingress_receipts, 2);
    assert_eq!(report_before.pending_projection_count, 2);
    assert_eq!(report_before.pending_derived_count, 2);
    assert_eq!(report_before.pending_index_count, 2);

    apply_replay_plan(&app);

    let report_after = app.doctor_report().expect("doctor after rebuild");
    assert_eq!(report_after.pending_projection_count, 0);
    assert_eq!(report_after.pending_derived_count, 0);
    assert_eq!(report_after.pending_index_count, 0);
    assert!(report_after.insights >= 2);
    assert!(report_after.verifications >= 1);

    let sessions = app.list_sessions().expect("sessions");
    let thread_id = sessions
        .iter()
        .find(|session| session.session_kind == "thread")
        .expect("thread session")
        .session_id
        .clone();
    let run_id = sessions
        .iter()
        .find(|session| session.session_kind == "run")
        .expect("run session")
        .session_id
        .clone();

    let thread = app.get_thread(&thread_id).expect("thread");
    assert_eq!(thread.entries.len(), 1);
    assert_eq!(thread.entries[0].entry.entry_kind, "selection_captured");
    assert_eq!(
        thread.entries[0].entry.text_body.as_deref(),
        Some("Use a narrow sink contract between relayd and AxiomSync.")
    );
    assert!(
        thread.entries[0]
            .anchors
            .iter()
            .any(|anchor| anchor.fingerprint.as_deref() == Some("sha1:dom:fp_001"))
    );

    let run = app.get_run(&run_id).expect("run");
    assert_eq!(run.entries.len(), 1);
    assert_eq!(run.entries[0].entry.entry_kind, "command_finished");
    assert_eq!(run.entries[0].artifacts.len(), 1);
    assert!(
        run.entries[0]
            .entry
            .text_body
            .as_deref()
            .is_some_and(|text| text
                .contains("HTTP contract fixture matched expected kernel response.")
                || text.contains("schema_validation: passed"))
    );

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let actor_kind: String = conn
        .query_row(
            "select actors.actor_kind
             from entries
             join actors on actors.actor_id = entries.actor_id
             where entries.entry_id = ?1",
            [thread.entries[0].entry.entry_id.as_str()],
            |row| row.get(0),
        )
        .expect("actor kind");
    assert_eq!(actor_kind, "assistant");

    let evidence_id = thread.entries[0].anchors[0].anchor_id.clone();
    let evidence = app.get_evidence(&evidence_id).expect("evidence");
    assert_eq!(evidence.anchor.anchor_id, evidence_id);
    assert_eq!(
        evidence.anchor.fingerprint.as_deref(),
        Some("sha1:dom:fp_001")
    );

    let document_id = run.entries[0].artifacts[0].artifact_id.clone();
    let document = app.get_document(&document_id).expect("document");
    assert_eq!(document.artifact.artifact_id, document_id);
    assert!(
        document.artifact.uri.contains("contract-fixture")
            || document.artifact.uri.contains("schema_validation")
    );

    let run_case = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| {
            case.problem
                .contains("HTTP contract fixture matched expected kernel response.")
        })
        .expect("run case");
    assert!(!run_case.verification.is_empty());
}

#[test]
fn reusable_derivations_require_evidence_anchors() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let request: AppendRawEventsRequest = serde_json::from_value(serde_json::json!({
        "batch_id": "no-anchor",
        "producer": "axiomrelay",
        "received_at_ms": 1710000100001i64,
        "events": [{
            "connector": "chatgpt_web_selection",
            "native_session_id": "chat:no-anchor",
            "native_event_id": "msg-no-anchor",
            "event_type": "selection_captured",
            "ts_ms": 1710000100000i64,
            "payload": {
                "session_kind": "thread",
                "page_title": "Empty selection"
            }
        }]
    }))
    .expect("request");

    let plan = app.plan_append_raw_events(request).expect("plan");
    app.apply_ingest_plan(&plan).expect("apply");
    apply_replay_plan(&app);

    let report = app.doctor_report().expect("doctor");
    assert_eq!(report.episodes, 1);
    assert_eq!(report.insights, 0);
    assert_eq!(report.verifications, 0);
    assert_eq!(report.claims, 0);
    assert_eq!(report.procedures, 0);
}

#[test]
fn final_form_fixtures_match_documented_sink_schema() {
    let schema = sink_schema();
    let validator = validator_for(&schema).expect("compile schema");

    for name in [
        "raw_event.chatgpt_selection.json",
        "raw_event.axiomrams_run_summary.json",
    ] {
        let fixture = fixture_value(name);
        let result = validator.validate(&fixture);
        assert!(
            result.is_ok(),
            "schema validation failed for {name}: {result:?}"
        );
    }
}

#[test]
fn documented_sink_schema_matches_domain_contract() {
    let schema = sink_schema();
    let append_required = schema["$defs"]["appendRawEventsRequest"]["required"]
        .as_array()
        .expect("append required");
    let cursor_required = schema["$defs"]["upsertSourceCursorRequest"]["required"]
        .as_array()
        .expect("cursor required");
    let event_types = schema["$defs"]["rawEvent"]["properties"]["event_type"]["enum"]
        .as_array()
        .expect("event enum");

    let expected_append = serde_json::json!(["batch_id", "producer", "received_at_ms", "events"]);
    let expected_cursor =
        serde_json::json!(["connector", "cursor_key", "cursor_value", "updated_at_ms"]);
    let expected_event_types = RAW_EVENT_TAXONOMY
        .iter()
        .map(|value| Value::String((*value).to_string()))
        .collect::<Vec<_>>();

    assert_eq!(
        append_required,
        expected_append.as_array().expect("expected append array")
    );
    assert_eq!(
        cursor_required,
        expected_cursor.as_array().expect("expected cursor array")
    );
    assert_eq!(event_types, &expected_event_types);
}

#[test]
fn normalized_receipts_use_canonical_connector_key() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let request = fixture_request("raw_event.chatgpt_selection.json");
    let plan = app.plan_append_raw_events(request).expect("plan ingest");
    let normalized = serde_json::from_str::<Value>(&plan.receipts[0].normalized_json)
        .expect("normalized receipt");

    assert_eq!(
        normalized.get("connector").and_then(Value::as_str),
        Some("chatgpt_web_selection")
    );
    assert!(normalized.get("connector_name").is_none());
}
