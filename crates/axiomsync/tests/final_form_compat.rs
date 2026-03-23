use std::fs;
use std::path::Path;

use axiomsync::domain::{
    AppendRawEventsRequest, SearchDocsRequest, SearchFilter, SearchInsightsRequest,
};
use jsonschema::validator_for;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

fn fixture_request(name: &str) -> AppendRawEventsRequest {
    serde_json::from_value(fixture_value(name)).expect("fixture request")
}

fn fixture_value(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../axiomsync-final-form-docs-package/examples")
        .join(name);
    serde_json::from_slice(&fs::read(path).expect("fixture file")).expect("fixture value")
}

fn sink_schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../axiomsync-final-form-docs-package/schema/kernel_sink_contract.json");
    serde_json::from_slice(&fs::read(path).expect("schema file")).expect("schema value")
}

#[test]
fn final_form_examples_are_accepted_projected_and_derived() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");

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

    app.rebuild().expect("rebuild");

    let report_after = app.doctor_report().expect("doctor after rebuild");
    assert_eq!(report_after.pending_projection_count, 0);
    assert_eq!(report_after.pending_derived_count, 0);
    assert_eq!(report_after.pending_index_count, 0);
    assert!(report_after.insights >= 2);
    assert!(report_after.verifications >= 1);

    let sessions = app.list_sessions().expect("sessions");
    let conversation_id = sessions
        .iter()
        .find(|session| session.session_kind == "conversation")
        .expect("conversation session")
        .session_id
        .clone();
    let run_id = sessions
        .iter()
        .find(|session| session.session_kind == "run")
        .expect("run session")
        .session_id
        .clone();

    let conversation = app.get_session(&conversation_id).expect("conversation");
    assert_eq!(conversation.entries.len(), 1);
    assert_eq!(conversation.entries[0].entry.entry_kind, "message");
    assert_eq!(
        conversation.entries[0].entry.text_body.as_deref(),
        Some("Use a narrow sink contract between relayd and AxiomSync.")
    );
    assert!(
        conversation.entries[0]
            .anchors
            .iter()
            .any(|anchor| anchor.fingerprint.as_deref() == Some("sha1:dom:fp_001"))
    );

    let run = app.get_session(&run_id).expect("run");
    assert_eq!(run.entries.len(), 1);
    assert_eq!(run.entries[0].entry.entry_kind, "check_result");
    assert_eq!(run.entries[0].artifacts.len(), 1);
    assert!(
        run.entries[0].entry.text_body.as_deref().is_some_and(
            |text| text.contains("HTTP contract fixture matched expected kernel response.")
        )
    );

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let actor_kind: String = conn
        .query_row(
            "select actors.actor_kind
             from entries
             join actors on actors.actor_id = entries.actor_id
             where entries.entry_id = ?1",
            [conversation.entries[0].entry.entry_id.as_str()],
            |row| row.get(0),
        )
        .expect("actor kind");
    assert_eq!(actor_kind, "assistant");

    let insight_hits = app
        .search_insights(SearchInsightsRequest {
            query: "narrow sink contract".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("search insights");
    assert!(!insight_hits.is_empty());
    assert!(!insight_hits[0].evidence.is_empty());

    let bundle = app
        .get_evidence_bundle("insight", &insight_hits[0].id)
        .expect("insight evidence bundle");
    assert_eq!(bundle.subject_kind, "insight");
    assert!(!bundle.evidence.is_empty());

    let docs_hits = app
        .search_docs(SearchDocsRequest {
            query: "kernel response".to_string(),
            limit: 10,
            filter: SearchFilter {
                workspace_root: Some("/workspace".to_string()),
                ..SearchFilter::default()
            },
        })
        .expect("search docs");
    assert!(!docs_hits.is_empty());
    assert!(docs_hits.iter().any(|hit| !hit.evidence.is_empty()));

    let run_case = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| {
            case.problem
                .contains("HTTP contract fixture matched expected kernel response.")
        })
        .expect("run case");
    assert!(
        run_case
            .verification
            .iter()
            .any(|verification| verification.status == "verified"
                && verification.method == "deterministic")
    );
}

#[test]
fn reusable_derivations_require_evidence_anchors() {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");
    let request: AppendRawEventsRequest = serde_json::from_value(serde_json::json!({
        "request_id": "no-anchor",
        "source": {
            "source_kind": "axiomrelay",
            "connector_name": "chatgpt_web_selection"
        },
        "events": [{
            "native_session_id": "chat:no-anchor",
            "native_entry_id": "msg-no-anchor",
            "event_type": "selection_captured",
            "observed_at_ms": 1710000100000i64,
            "captured_at_ms": 1710000100001i64,
            "payload": {
                "page_title": "Empty selection"
            },
            "hints": {
                "session_kind": "conversation",
                "entry_kind": "message"
            }
        }]
    }))
    .expect("request");

    let plan = app.plan_append_raw_events(request).expect("plan");
    app.apply_ingest_plan(&plan).expect("apply");
    app.rebuild().expect("rebuild");

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
