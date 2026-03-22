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
            connector: "codex".to_string(),
            native_schema_version: None,
            native_session_id: "session-1".to_string(),
            native_event_id: Some("1".to_string()),
            event_type: "user_message".to_string(),
            ts_ms: 1,
            payload: json!({"workspace_root": "/repo/app", "turn_id": "t1", "actor": "user", "text": "timeout error"}),
        }],
        cursor: None,
    };
    let plan = app.plan_ingest(&batch).expect("plan");
    app.apply_ingest(&plan).expect("apply");
    let project = app.plan_projection().expect("projection");
    app.apply_projection(&project).expect("apply projection");
    let derive = app.plan_derivation().expect("derive");
    app.apply_derivation(&derive).expect("apply derive");
    let grant_plan = app
        .plan_workspace_token_grant("/repo/app", "secret-token")
        .expect("grant plan");
    app.apply_workspace_token_grant(&grant_plan)
        .expect("grant apply");
    app
}

fn app_with_two_workspaces() -> AxiomSync {
    let app = app_with_data();
    let second_batch = ConnectorBatchInput {
        events: vec![RawEventInput {
            connector: "claude_code".to_string(),
            native_schema_version: None,
            native_session_id: "session-2".to_string(),
            native_event_id: Some("2".to_string()),
            event_type: "user_message".to_string(),
            ts_ms: 2,
            payload: json!({"workspace_root": "/repo/other", "turn_id": "t2", "actor": "user", "text": "timeout error elsewhere"}),
        }],
        cursor: None,
    };
    let ingest = app.plan_ingest(&second_batch).expect("plan second");
    app.apply_ingest(&ingest).expect("apply second");
    let replay = app.plan_replay().expect("replay second");
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
async fn web_page_and_mcp_endpoint_render() {
    let app = app_with_data();
    let router = router(app.clone());

    let page = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("authorization", "Bearer secret-token")
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
    let app = app_with_data();
    let runbook = app.list_runbooks().expect("runbooks").remove(0);

    let resources = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list"
        }),
        runbook.workspace_id.as_deref(),
    )
    .expect("resources");
    let tools = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }),
        runbook.workspace_id.as_deref(),
    )
    .expect("tools");
    let episode = mcp::handle_request(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/read",
            "params": {"uri": format!("axiom://episode/{}", runbook.episode_id)}
        }),
        runbook.workspace_id.as_deref(),
    )
    .expect("episode");

    let resource_rows = resources["result"].as_array().expect("resource array");
    assert!(
        resource_rows
            .iter()
            .any(|row| row["uri"] == "axiom://episode/{id}")
    );
    let tool_rows = tools["result"].as_array().expect("tool array");
    assert!(tool_rows.iter().any(|row| row["name"] == "search_commands"));
    assert_eq!(episode["result"]["episode_id"], runbook.episode_id);
    assert_eq!(episode["result"]["problem"], runbook.problem);
    assert!(episode["result"]["evidence"].is_array());
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
