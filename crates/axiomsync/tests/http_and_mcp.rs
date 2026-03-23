use std::sync::Arc;

use axiomsync::domain::{
    ConnectorBatchInput, EpisodeExtraction, RawEventInput, VerificationExtraction,
    VerificationKind, VerificationStatus,
};
use axiomsync::http_api::router;
use axiomsync::kernel::AxiomSync;
use axiomsync::llm::MockLlmClient;
use axiomsync::mcp;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tempfile::tempdir;
use tower::ServiceExt;

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

fn app_with_data() -> AxiomSync {
    let temp = tempdir().expect("tempdir");
    let root = temp.keep();
    let app = axiomsync::open_with_llm(
        root,
        Arc::new(MockLlmClient {
            extraction: EpisodeExtraction {
                problem: "Timeout error".to_string(),
                root_cause: Some("queue drift".to_string()),
                fix: Some("restart worker".to_string()),
                commands: vec!["cargo test".to_string()],
                decisions: vec![],
                snippets: vec![],
            },
            verifications: vec![VerificationExtraction {
                kind: VerificationKind::Test,
                status: VerificationStatus::Pass,
                summary: Some("passed".to_string()),
                evidence: Some("passed".to_string()),
                pass_condition: Some("mock llm".to_string()),
                exit_code: Some(0),
                human_confirmed: false,
            }],
        }),
    )
    .expect("app");
    app.init().expect("init");
    let batch = ConnectorBatchInput {
        events: vec![RawEventInput {
            source: "codex".to_string(),
            native_schema_version: None,
            native_session_id: "session-1".to_string(),
            native_event_id: Some("1".to_string()),
            event_type: "user_message".to_string(),
            ts_ms: 1,
            payload: json!({"workspace_root": "/repo/app", "turn_id": "t1", "actor": "user", "text": "timeout error"}),
        }],
        cursor: None,
    };
    let plan = plan_ingest(&app, &batch).expect("plan");
    app.apply_ingest(&plan).expect("apply");
    let raw_events = app.load_raw_events().expect("raw events");
    let project = app.plan_projection(&raw_events).expect("projection");
    app.apply_projection(&project).expect("apply projection");
    let derive = plan_derivation(&app).expect("derive");
    app.apply_derivation(&derive).expect("apply derive");
    let grant_plan = app
        .plan_workspace_token_grant("/repo/app", "secret-token")
        .expect("grant plan");
    app.apply_workspace_token_grant(&grant_plan)
        .expect("grant apply");
    let admin_plan = app
        .plan_admin_token_grant("admin-secret-token")
        .expect("admin grant plan");
    app.apply_admin_token_grant(&admin_plan)
        .expect("admin grant apply");
    app
}

fn app_with_runtime_and_document_data() -> AxiomSync {
    let app = app_with_data();
    let batch = ConnectorBatchInput {
        events: vec![
            RawEventInput {
                source: "codex".to_string(),
                native_schema_version: Some("agent-record-v1".to_string()),
                native_session_id: "runtime-session".to_string(),
                native_event_id: Some("run-evt".to_string()),
                event_type: "task_state".to_string(),
                ts_ms: 3,
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
                native_event_id: Some("doc-evt".to_string()),
                event_type: "document_snapshot".to_string(),
                ts_ms: 4,
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
    let plan = plan_ingest(&app, &batch).expect("plan runtime");
    app.apply_ingest(&plan).expect("apply runtime");
    let replay = plan_replay(&app).expect("replay runtime");
    app.apply_replay(&replay).expect("apply replay runtime");
    app
}

fn app_with_two_workspaces() -> AxiomSync {
    let app = app_with_data();
    let second_batch = ConnectorBatchInput {
        events: vec![RawEventInput {
            source: "claude_code".to_string(),
            native_schema_version: None,
            native_session_id: "session-2".to_string(),
            native_event_id: Some("2".to_string()),
            event_type: "user_message".to_string(),
            ts_ms: 2,
            payload: json!({"workspace_root": "/repo/other", "turn_id": "t2", "actor": "user", "text": "timeout error elsewhere"}),
        }],
        cursor: None,
    };
    let ingest = plan_ingest(&app, &second_batch).expect("plan second");
    app.apply_ingest(&ingest).expect("apply second");
    let replay = plan_replay(&app).expect("replay second");
    app.apply_replay(&replay).expect("apply replay second");
    app
}

#[tokio::test]
async fn api_requires_bearer_token_and_accepts_valid_scope() {
    let app = app_with_data();
    let router = router(app.clone());

    let unauthorized = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/episodes?query=timeout")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let runbook = app.list_runbooks().expect("runbooks").remove(0);
    let authorized = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/episodes?query=timeout&workspace_id={}",
                    runbook.workspace_id.clone().expect("workspace")
                ))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let authorized_status = authorized.status();
    let authorized_body = axum::body::to_bytes(authorized.into_body(), usize::MAX)
        .await
        .expect("body");
    assert_eq!(
        authorized_status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&authorized_body)
    );
}

#[tokio::test]
async fn admin_routes_require_global_admin_token() {
    let app = app_with_data();
    let router = router(app);

    let workspace_token_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/project/rebuild/plan")
                .header("authorization", "Bearer secret-token")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(workspace_token_response.status(), StatusCode::UNAUTHORIZED);

    let admin_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/project/rebuild/plan")
                .header("authorization", "Bearer admin-secret-token")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(admin_response.status(), StatusCode::OK);

    let index_response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/")
                .header("authorization", "Bearer admin-secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(index_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn search_accepts_source_filter_over_http_and_mcp() {
    let app = app_with_two_workspaces();
    let router = router(app.clone());
    let workspace_id = app
        .list_runbooks()
        .expect("runbooks")
        .into_iter()
        .find(|runbook| runbook.workspace_id.is_some())
        .and_then(|runbook| runbook.workspace_id)
        .expect("workspace id");

    let http = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/episodes?query=timeout&source=codex&workspace_id={workspace_id}"
                ))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(http.status(), StatusCode::OK);
    let http_body = axum::body::to_bytes(http.into_body(), usize::MAX)
        .await
        .expect("body");
    let http_value: serde_json::Value = serde_json::from_slice(&http_body).expect("json");
    assert_eq!(http_value.as_array().map(Vec::len), Some(1), "{http_value}");
    assert_eq!(http_value[0]["source"].as_str(), Some("codex"));

    let mcp_response = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "workspace_id": workspace_id.clone(),
                "name": "search_episodes",
                "arguments": {
                    "query": "timeout",
                    "filters": {
                        "workspace_id": workspace_id.clone(),
                        "source": "codex"
                    }
                }
            }
        }),
        Some(&workspace_id),
    )
    .expect("mcp response");
    let result = &mcp_response["result"];
    assert_eq!(result.as_array().map(Vec::len), Some(1), "{result}");
    assert_eq!(result[0]["source"].as_str(), Some("codex"));
}

#[tokio::test]
async fn web_page_and_mcp_endpoint_render() {
    let app = app_with_data();
    let router = router(app.clone());

    let page = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("authorization", "Bearer admin-secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let page_status = page.status();
    let page_body = axum::body::to_bytes(page.into_body(), usize::MAX)
        .await
        .expect("body");
    assert_eq!(
        page_status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&page_body)
    );

    let runbook = app.list_runbooks().expect("runbooks").remove(0);
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "workspace_id": runbook.workspace_id.clone().expect("workspace"),
            "name": "search_episodes",
            "arguments": {"query": "timeout", "limit": 5, "filters": {"workspace_id": runbook.workspace_id}}
        }
    });
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header("authorization", "Bearer secret-token")
                .header("content-type", "application/json")
                .body(Body::from(request.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let response_status = response.status();
    let response_body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    assert_eq!(
        response_status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&response_body)
    );
}

#[tokio::test]
async fn mcp_http_rejects_cross_workspace_request() {
    let app = app_with_data();
    let router = router(app);
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search_episodes",
            "arguments": {
                "query": "timeout",
                "filters": {"workspace_id": "ws_other"}
            }
        }
    });
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header("authorization", "Bearer secret-token")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn mcp_stdio_binding_rejects_out_of_scope_resource() {
    let app = app_with_data();
    let episode_id = app.list_runbooks().expect("runbooks").remove(0).episode_id;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": {"uri": format!("axiom://episode/{episode_id}")}
    });
    let error = mcp::handle_request(&app, request, Some("ws_other")).expect_err("must reject");
    assert!(error.to_string().contains("outside bound workspace"));
}

#[test]
fn mcp_roots_list_honors_workspace_binding() {
    let app = app_with_two_workspaces();
    let runbook = app.list_runbooks().expect("runbooks").remove(0);
    let response = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "roots/list"
        }),
        runbook.workspace_id.as_deref(),
    )
    .expect("roots");
    let roots = response["result"].as_array().expect("roots array");
    assert_eq!(roots.len(), 1);
    assert_eq!(
        roots[0]["workspace_id"].as_str(),
        runbook.workspace_id.as_deref()
    );
}

#[tokio::test]
async fn mcp_http_binds_unscoped_tool_calls_to_authorized_workspace() {
    let app = app_with_two_workspaces();
    let router = router(app.clone());
    let runbook = app.list_runbooks().expect("runbooks").remove(0);
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header("authorization", "Bearer secret-token")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tools/call",
                        "params": {
                            "name": "search_episodes",
                            "arguments": {"query": "timeout", "limit": 10}
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let rows = value["result"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["workspace_id"].as_str(),
        runbook.workspace_id.as_deref()
    );
}

#[test]
fn mcp_lists_resources_tools_and_reads_episode_payload() {
    let app = app_with_runtime_and_document_data();
    let case = app.list_cases().expect("cases").remove(0);
    let run = app.list_runs(None).expect("runs").remove(0);
    let document = app
        .list_documents(None, Some("mission"))
        .expect("documents")
        .remove(0);

    let resources = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list"
        }),
        case.workspace_id.as_deref(),
    )
    .expect("resources");
    let tools = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }),
        case.workspace_id.as_deref(),
    )
    .expect("tools");
    let case_resource = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/read",
            "params": {"uri": format!("axiom://cases/{}", case.case_id)}
        }),
        case.workspace_id.as_deref(),
    )
    .expect("case");
    let run_resource = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "resources/read",
            "params": {"uri": format!("axiom://runs/{}", run.stable_id)}
        }),
        case.workspace_id.as_deref(),
    )
    .expect("run");
    let document_resource = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "resources/read",
            "params": {"uri": format!("axiom://documents/{}", document.stable_id)}
        }),
        case.workspace_id.as_deref(),
    )
    .expect("document");

    let resource_rows = resources["result"].as_array().expect("resource array");
    assert!(
        resource_rows
            .iter()
            .any(|row| row["uri"] == "axiom://cases/{id}")
    );
    let tool_rows = tools["result"].as_array().expect("tool array");
    assert!(tool_rows.iter().any(|row| row["name"] == "search_cases"));
    assert!(tool_rows.iter().any(|row| row["name"] == "search_commands"));
    assert!(tool_rows.iter().any(|row| row["name"] == "get_run"));
    assert_eq!(case_resource["result"]["case_id"], case.case_id);
    assert_eq!(case_resource["result"]["problem"], case.problem);
    assert_eq!(run_resource["result"]["run"]["run_id"], "run-1");
    assert_eq!(
        document_resource["result"]["document"]["document_id"],
        "mission-doc"
    );
    assert!(case_resource["result"]["evidence"].is_array());
}

#[tokio::test]
async fn canonical_http_surfaces_cases_runs_and_documents() {
    let app = app_with_runtime_and_document_data();
    let router = router(app.clone());
    let case = app.list_cases().expect("cases").remove(0);
    let run = app.list_runs(None).expect("runs").remove(0);
    let task = app
        .get_run(&run.stable_id)
        .expect("run")
        .tasks
        .remove(0)
        .task;
    let document = app.list_documents(None, None).expect("documents").remove(0);
    let workspace_id = case.workspace_id.clone().expect("workspace");

    let cases = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/cases?query=timeout&workspace_id={workspace_id}"
                ))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(cases.status(), StatusCode::OK);
    let cases_body = axum::body::to_bytes(cases.into_body(), usize::MAX)
        .await
        .expect("body");
    let cases_value: serde_json::Value = serde_json::from_slice(&cases_body).expect("json");
    assert_eq!(
        cases_value[0]["case_id"].as_str(),
        Some(case.case_id.as_str())
    );

    let run_detail = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/runs/{}", run.stable_id))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(run_detail.status(), StatusCode::OK);
    let run_body = axum::body::to_bytes(run_detail.into_body(), usize::MAX)
        .await
        .expect("body");
    let run_value: serde_json::Value = serde_json::from_slice(&run_body).expect("json");
    assert_eq!(run_value["run"]["run_id"].as_str(), Some("run-1"));

    let task_detail = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/tasks/{}", task.stable_id))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(task_detail.status(), StatusCode::OK);

    let documents = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/documents?workspace_id={workspace_id}&kind=mission"
                ))
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(documents.status(), StatusCode::OK);
    let documents_body = axum::body::to_bytes(documents.into_body(), usize::MAX)
        .await
        .expect("body");
    let documents_value: serde_json::Value = serde_json::from_slice(&documents_body).expect("json");
    assert_eq!(
        documents_value[0]["stable_id"].as_str(),
        Some(document.stable_id.as_str())
    );
}

#[test]
fn mcp_bound_search_commands_stays_in_workspace() {
    let app = app_with_two_workspaces();
    let runbook = app.list_runbooks().expect("runbooks").remove(0);
    let response = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "search_commands",
                "arguments": {"query": "cargo", "limit": 10}
            }
        }),
        runbook.workspace_id.as_deref(),
    )
    .expect("search commands");
    let rows = response["result"].as_array().expect("rows");
    assert!(!rows.is_empty());
    let expected = app
        .search_commands_in_workspace(
            "cargo",
            10,
            runbook.workspace_id.as_deref().expect("workspace"),
        )
        .expect("expected");
    assert_eq!(rows.len(), expected.len());
    for (row, expected_row) in rows.iter().zip(expected.iter()) {
        assert_eq!(row["episode_id"], expected_row.episode_id);
        assert_eq!(row["command"], expected_row.command);
    }
}
