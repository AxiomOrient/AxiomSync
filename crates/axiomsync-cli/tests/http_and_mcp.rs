use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use axiomsync_domain::{
    AppendRawEventsRequest, DerivePlan, ProjectionPlan, ReplayPlan, SearchHit, workspace_stable_id,
};
use axiomsync_kernel::AxiomSync;
use tempfile::tempdir;

fn legacy(parts: &[&str]) -> String {
    parts.concat()
}

struct SeededApp {
    app: AxiomSync,
    workspace_token: String,
    admin_token: String,
    case_id: String,
    case_problem: String,
    thread_id: String,
    task_id: String,
    run_id: String,
    document_id: String,
    evidence_id: String,
}

fn apply_replay_plan(app: &AxiomSync) {
    let plan = app.build_replay_plan().expect("replay plan");
    app.apply_replay(&plan).expect("apply replay plan");
}

async fn decode_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

async fn decode_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

fn seed_request() -> AppendRawEventsRequest {
    serde_json::from_value(serde_json::json!({
        "batch_id": "req-http",
        "producer": "relay",
        "received_at_ms": 1710000004000i64,
        "events": [
            {
                "connector": "chatgpt_web_selection",
                "native_session_id": "thread-http",
                "native_event_id": "evt-http",
                "event_type": "selection_captured",
                "ts_ms": 1710000001000i64,
                "payload": {
                    "session_kind": "thread",
                    "workspace_root": "/workspace/http",
                    "page_title": "HTTP Thread",
                    "page_url": "https://chatgpt.com/c/http",
                    "selection": {
                        "text": "Root cause: queue drift\nFix: rebuild projection\nDecision: keep sink narrow\n$ cargo test -q",
                        "start_hint": "Root cause: queue drift",
                        "end_hint": "$ cargo test -q",
                        "dom_fingerprint": "sha1:http:selection"
                    },
                    "source_message": {
                        "message_id": "msg-http",
                        "role": "assistant"
                    }
                },
                "artifacts": [{
                    "artifact_kind": "file",
                    "uri": "file:///workspace/http/src/lib.rs",
                    "mime_type": "text/rust"
                }]
            },
            {
                "connector": "work_state_export",
                "native_session_id": "task-http",
                "native_event_id": "evt-task",
                "event_type": "task_state_imported",
                "ts_ms": 1710000002000i64,
                "payload": {
                    "session_kind": "task",
                    "workspace_root": "/workspace/http",
                    "title": "HTTP Task",
                    "text": "Task running"
                }
            },
            {
                "connector": "cli_local_exec",
                "native_session_id": "run-http",
                "native_event_id": "evt-run",
                "event_type": "command_finished",
                "ts_ms": 1710000003000i64,
                "payload": {
                    "session_kind": "run",
                    "workspace_root": "/workspace/http",
                    "title": "HTTP Run",
                    "summary": "Run finished",
                    "command": {
                        "argv": ["cargo", "test", "-q"],
                        "cwd": "/workspace/http",
                        "exit_code": 0,
                        "duration_ms": 250
                    },
                    "checks": [{
                        "name": "cargo_test",
                        "status": "passed"
                    }]
                }
            },
            {
                "connector": "work_state_export",
                "native_session_id": "import-http",
                "native_event_id": "evt-doc",
                "event_type": "artifact_emitted",
                "ts_ms": 1710000004000i64,
                "payload": {
                    "session_kind": "import",
                    "workspace_root": "/workspace/http",
                    "title": "Imported doc",
                    "text": "Imported document body"
                },
                "artifacts": [{
                    "artifact_kind": "document",
                    "uri": "file:///workspace/http/docs/guide.md",
                    "mime_type": "text/markdown"
                }]
            }
        ]
    }))
    .expect("request")
}

fn seed_app() -> SeededApp {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync_cli::open(temp.path()).expect("app");
    let ingest = app
        .plan_append_raw_events(seed_request())
        .expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    apply_replay_plan(&app);

    let workspace_token = "http-workspace-token".to_string();
    let admin_token = "http-admin-token".to_string();
    let workspace_plan = app
        .plan_workspace_token_grant("/workspace/http", &workspace_token)
        .expect("workspace grant plan");
    app.apply_workspace_token_grant(&workspace_plan)
        .expect("workspace grant");
    let admin_plan = app
        .plan_admin_token_grant(&admin_token)
        .expect("admin grant plan");
    app.apply_admin_token_grant(&admin_plan)
        .expect("admin grant");

    let sessions = app.list_sessions().expect("sessions");
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
    let run_id = sessions
        .iter()
        .find(|session| session.session_kind == "run")
        .expect("run")
        .session_id
        .clone();
    let thread = app.get_thread(&thread_id).expect("thread");
    let evidence_id = thread.entries[0].anchors[0].anchor_id.clone();
    let case = app
        .list_cases()
        .expect("cases")
        .into_iter()
        .find(|case| case.workspace_root.as_deref() == Some("/workspace/http"))
        .expect("case");
    let case_id = case.case_id.clone();
    let case_problem = case.problem.clone();
    let document_id = app
        .list_documents(Some("/workspace/http"), Some("document"))
        .expect("documents")[0]
        .artifact
        .artifact_id
        .clone();

    std::mem::forget(temp);
    SeededApp {
        app,
        workspace_token,
        admin_token,
        case_id,
        case_problem,
        thread_id,
        task_id,
        run_id,
        document_id,
        evidence_id,
    }
}

#[tokio::test]
async fn canonical_http_routes_work_with_auth() {
    let seeded = seed_app();
    let router = axiomsync_http::router(seeded.app.clone());

    let health = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(health.status(), StatusCode::OK);

    let index = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(index.status(), StatusCode::OK);
    let index_html = decode_text(index).await;
    assert!(index_html.contains("Cases"));
    assert!(index_html.contains("Threads"));

    let removed_case_page = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/cases/{}", seeded.case_id))
                .header(
                    "authorization",
                    format!("Bearer {}", seeded.workspace_token),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(removed_case_page.status(), StatusCode::NOT_FOUND);

    for uri in [
        format!("/api/cases/{}", seeded.case_id),
        format!("/api/threads/{}", seeded.thread_id),
        format!("/api/runs/{}", seeded.run_id),
        format!("/api/tasks/{}", seeded.task_id),
        format!("/api/documents/{}", seeded.document_id),
        format!("/api/evidence/{}", seeded.evidence_id),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(
                        "authorization",
                        format!("Bearer {}", seeded.workspace_token),
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let search_cases = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/query/search-cases")
                .method("POST")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", seeded.workspace_token),
                )
                .body(Body::from(
                    serde_json::json!({
                        "query": seeded.case_problem,
                        "limit": 10,
                        "filter": { "workspace_root": "/workspace/http" }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(search_cases.status(), StatusCode::OK);
    let hits: Vec<SearchHit> = decode_json(search_cases).await;
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|hit| hit.kind == "case"));

    let run_list = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/runs")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(run_list.status(), StatusCode::OK);

    let projection_plan_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/projection/plan")
                .method("POST")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(projection_plan_response.status(), StatusCode::OK);
    let projection_plan: ProjectionPlan = decode_json(projection_plan_response).await;

    let projection_apply = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/projection/apply")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::from(
                    serde_json::to_vec(&projection_plan).expect("projection plan json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(projection_apply.status(), StatusCode::OK);

    let derivation_plan_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/derivations/plan")
                .method("POST")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(derivation_plan_response.status(), StatusCode::OK);
    let derivation_plan: DerivePlan = decode_json(derivation_plan_response).await;

    let derivation_apply = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/derivations/apply")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::from(
                    serde_json::to_vec(&derivation_plan).expect("derivation plan json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(derivation_apply.status(), StatusCode::OK);

    let replay_plan_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/replay/plan")
                .method("POST")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(replay_plan_response.status(), StatusCode::OK);
    let replay_plan: ReplayPlan = decode_json(replay_plan_response).await;

    let replay_apply = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin/replay/apply")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::from(
                    serde_json::to_vec(&replay_plan).expect("replay plan json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(replay_apply.status(), StatusCode::OK);

    for path in [
        "/admin/rebuild/projection",
        "/admin/rebuild/derivations",
        "/admin/rebuild/index",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .method("POST")
                    .header("authorization", format!("Bearer {}", seeded.admin_token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    assert_eq!(
        seeded
            .app
            .authorize_workspace(
                &seeded.workspace_token,
                Some(&workspace_stable_id("/workspace/http"))
            )
            .expect("authorized"),
        Some(workspace_stable_id("/workspace/http"))
    );
}

#[test]
fn mcp_exposes_only_canonical_resources_and_tools() {
    let seeded = seed_app();
    let workspace_id = workspace_stable_id("/workspace/http");

    for uri in [
        format!("axiom://cases/{}", seeded.case_id),
        format!("axiom://threads/{}", seeded.thread_id),
        format!("axiom://runs/{}", seeded.run_id),
        format!("axiom://tasks/{}", seeded.task_id),
        format!("axiom://documents/{}", seeded.document_id),
        format!("axiom://evidence/{}", seeded.evidence_id),
    ] {
        let response = axiomsync_mcp::handle_request(
            &seeded.app,
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": { "uri": uri }
            }),
            Some(&workspace_id),
        )
        .expect("resource");
        assert!(response.get("result").is_some());
    }

    let resources = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"resources/list"}),
        Some(&workspace_id),
    )
    .expect("resources");
    let resource_uris = resources["result"]["resources"]
        .as_array()
        .expect("resources array")
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(resource_uris.iter().any(|uri| uri == "axiom://cases/{id}"));
    assert!(
        resource_uris
            .iter()
            .any(|uri| uri == "axiom://threads/{id}")
    );
    let legacy_session_uri = legacy(&["session", "://"]);
    assert!(
        resource_uris
            .iter()
            .all(|uri| !uri.contains(legacy_session_uri.as_str()))
    );
    assert!(
        resource_uris
            .iter()
            .all(|uri| !uri.contains(legacy(&["axiom://", "sessions/"]).as_str()))
    );
    assert!(resource_uris.iter().all(|uri| !uri.contains("runbook")));

    let tools = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
        Some(&workspace_id),
    )
    .expect("tools");
    let tool_names = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    for name in [
        "search_cases",
        "get_case",
        "get_thread",
        "get_run",
        "get_task",
        "get_document",
        "get_evidence",
        "list_runs",
        "list_documents",
    ] {
        assert!(
            tool_names.iter().any(|tool| tool == name),
            "missing tool {name}"
        );
    }
    let unexpected_tools = [
        legacy(&["search", "_", "entries"]),
        legacy(&["search", "_", "episodes"]),
        legacy(&["get", "_", "runbook"]),
        legacy(&["find", "_", "runbook"]),
    ];
    for name in &unexpected_tools {
        assert!(
            tool_names.iter().all(|tool| tool != name),
            "unexpected tool {name}"
        );
    }

    let task_tool = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "get_task", "arguments": { "id": seeded.task_id } }
        }),
        Some(&workspace_id),
    )
    .expect("task tool");
    assert!(task_tool.get("result").is_some());

    let removed_alias = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": { "name": "get_task", "arguments": { "task_id": seeded.task_id } }
        }),
        Some(&workspace_id),
    );
    assert!(removed_alias.is_err());

    let search_cases = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "search_cases",
                "arguments": {
                    "query": seeded.case_problem,
                    "limit": 10,
                    "filter": { "workspace_root": "/workspace/http" }
                }
            }
        }),
        Some(&workspace_id),
    )
    .expect("search cases tool");
    let hits = search_cases["result"].as_array().expect("hits array");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .all(|hit| hit.get("kind").and_then(|value| value.as_str()) == Some("case"))
    );

    let list_documents = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "list_documents",
                "arguments": {
                    "workspace_root": "/workspace/http",
                    "kind": "document"
                }
            }
        }),
        Some(&workspace_id),
    )
    .expect("list documents tool");
    assert!(list_documents.get("result").is_some());

    let unknown_legacy_tool = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": legacy(&["get", "_", "runbook"]),
                "arguments": { "id": seeded.case_id }
            }
        }),
        Some(&workspace_id),
    )
    .expect("legacy tool response");
    assert!(unknown_legacy_tool.get("error").is_some());
}

#[tokio::test]
async fn route_auth_and_loopback_matrix_is_enforced() {
    let seeded = seed_app();
    let router = axiomsync_http::router(seeded.app.clone());

    let missing_workspace_auth = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/cases/{}", seeded.case_id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(missing_workspace_auth.status(), StatusCode::FORBIDDEN);

    let workspace_on_admin_route = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/runs")
                .header(
                    "authorization",
                    format!("Bearer {}", seeded.workspace_token),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(workspace_on_admin_route.status(), StatusCode::FORBIDDEN);

    let admin_on_workspace_route = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/cases/{}", seeded.case_id))
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(admin_on_workspace_route.status(), StatusCode::FORBIDDEN);

    let loopback_request = Request::builder()
        .uri("/sink/raw-events/plan")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&seed_request()).expect("seed request json"),
        ))
        .expect("loopback request");
    let mut loopback_request = loopback_request;
    loopback_request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            4400,
        )));
    let loopback_response = router
        .clone()
        .oneshot(loopback_request)
        .await
        .expect("loopback response");
    assert_eq!(loopback_response.status(), StatusCode::OK);

    let denied_request = Request::builder()
        .uri("/sink/raw-events/plan")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&seed_request()).expect("seed request json"),
        ))
        .expect("non-loopback request");
    let mut denied_request = denied_request;
    denied_request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)),
            4400,
        )));
    let denied_response = router
        .oneshot(denied_request)
        .await
        .expect("denied response");
    assert_eq!(denied_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn http_and_mcp_return_equivalent_search_results() {
    let seeded = seed_app();
    let router = axiomsync_http::router(seeded.app.clone());
    let workspace_id = workspace_stable_id("/workspace/http");

    let http_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/query/search-cases")
                .method("POST")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", seeded.workspace_token),
                )
                .body(Body::from(
                    serde_json::json!({
                        "query": seeded.case_problem,
                        "limit": 10,
                        "filter": { "workspace_root": "/workspace/http" }
                    })
                    .to_string(),
                ))
                .expect("http request"),
        )
        .await
        .expect("http response");
    assert_eq!(http_response.status(), StatusCode::OK);
    let http_hits: Vec<SearchHit> = decode_json(http_response).await;

    let mcp_response = axiomsync_mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": {
                "name": "search_cases",
                "arguments": {
                    "query": seeded.case_problem,
                    "limit": 10,
                    "filter": { "workspace_root": "/workspace/http" }
                }
            }
        }),
        Some(&workspace_id),
    )
    .expect("mcp response");
    let mcp_hits: Vec<SearchHit> =
        serde_json::from_value(mcp_response["result"].clone()).expect("mcp hits");

    assert_eq!(http_hits.len(), mcp_hits.len());
    assert_eq!(http_hits[0].id, mcp_hits[0].id);
    assert_eq!(http_hits[0].title, mcp_hits[0].title);
}
