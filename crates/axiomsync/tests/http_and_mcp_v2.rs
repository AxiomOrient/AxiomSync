use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use axiomsync::domain::{AppendRawEventsRequest, SearchClaimsRequest, SearchProceduresRequest, SearchFilter, workspace_stable_id};
use tempfile::tempdir;

struct SeededApp {
    app: axiomsync::AxiomSync,
    workspace_token: String,
    admin_token: String,
    session_id: String,
    entry_id: String,
    artifact_id: String,
    anchor_id: String,
    episode_id: String,
    claim_id: String,
    procedure_id: String,
    task_id: String,
    run_id: String,
    document_id: String,
    evidence_id: String,
}

fn seed_app() -> SeededApp {
    let temp = tempdir().expect("tempdir");
    let app = axiomsync::open(temp.path()).expect("app");
    let request: AppendRawEventsRequest = serde_json::from_value(serde_json::json!({
        "request_id": "req-http",
        "events": [
            {
                "source": "relay",
                "native_session_id": "session-http",
                "native_event_id": "evt-http",
                "event_type": "assistant_message",
                "ts_ms": 1710000001000i64,
                "workspace_root": "/workspace/http",
                "payload": {
                    "title": "HTTP Session",
                    "text": "Root cause: queue drift\nFix: rebuild projection\nDecision: keep sink narrow\n$ cargo test -q"
                },
                "artifacts": [{
                    "artifact_kind": "file",
                    "uri": "file:///workspace/http/src/lib.rs",
                    "mime_type": "text/rust"
                }]
            },
            {
                "source": "relay",
                "session_kind": "task",
                "external_session_key": "task-http",
                "external_entry_key": "evt-task",
                "event_kind": "task_update",
                "observed_at": "2024-03-09T16:00:02Z",
                "workspace_root": "/workspace/http",
                "payload": {
                    "title": "HTTP Task",
                    "text": "Task running"
                }
            },
            {
                "source": "relay",
                "session_kind": "run",
                "external_session_key": "run-http",
                "external_entry_key": "evt-run",
                "event_kind": "run_update",
                "observed_at": "2024-03-09T16:00:03Z",
                "workspace_root": "/workspace/http",
                "payload": {
                    "title": "HTTP Run",
                    "text": "Run finished"
                }
            },
            {
                "source": "relay",
                "session_kind": "import",
                "external_session_key": "import-http",
                "external_entry_key": "evt-doc",
                "event_kind": "document_snapshot",
                "observed_at": "2024-03-09T16:00:04Z",
                "workspace_root": "/workspace/http",
                "payload": {
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
    .expect("request");
    let ingest = app.plan_append_raw_events(request).expect("plan ingest");
    app.apply_ingest_plan(&ingest).expect("apply ingest");
    app.rebuild().expect("rebuild");

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
    let session_id = sessions
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
    let run_id = sessions
        .iter()
        .find(|session| session.session_kind == "run")
        .expect("run")
        .session_id
        .clone();
    let session = app.get_session(&session_id).expect("session");
    let entry_id = session.entries[0].entry.entry_id.clone();
    let artifact_id = session.entries[0].artifacts[0].artifact_id.clone();
    let anchor_id = session.entries[0].anchors[0].anchor_id.clone();
    let episode_id = app.list_cases().expect("cases")[0].case_id.clone();
    let claim_id = app
        .search_claims(SearchClaimsRequest {
            query: "root cause".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("claims")[0]
        .id
        .clone();
    let procedure_id = app
        .search_procedures(SearchProceduresRequest {
            query: "cargo test".to_string(),
            limit: 10,
            filter: SearchFilter::default(),
        })
        .expect("procedures")[0]
        .id
        .clone();
    let imported = app
        .list_documents(Some("/workspace/http"), Some("document"))
        .expect("documents");
    let document_id = imported[0].artifact.artifact_id.clone();

    std::mem::forget(temp);
    SeededApp {
        app,
        workspace_token,
        admin_token,
        session_id,
        entry_id,
        artifact_id: artifact_id.clone(),
        anchor_id: anchor_id.clone(),
        episode_id,
        claim_id,
        procedure_id,
        task_id,
        run_id,
        document_id,
        evidence_id: anchor_id,
    }
}

#[tokio::test]
async fn canonical_and_compat_http_routes_work_with_auth() {
    let seeded = seed_app();
    let router = axiomsync::http_api::router(seeded.app.clone());

    for uri in [
        format!("/api/sessions/{}", seeded.session_id),
        format!("/api/entries/{}", seeded.entry_id),
        format!("/api/artifacts/{}", seeded.artifact_id),
        format!("/api/anchors/{}", seeded.anchor_id),
        format!("/api/episodes/{}", seeded.episode_id),
        format!("/api/claims/{}", seeded.claim_id),
        format!("/api/cases/{}", seeded.episode_id),
        format!("/api/threads/{}", seeded.session_id),
        format!("/api/runbooks/{}", seeded.episode_id),
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
                    .header("authorization", format!("Bearer {}", seeded.workspace_token))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let procedure_response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/procedures/{}", seeded.procedure_id))
                .header("authorization", format!("Bearer {}", seeded.admin_token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(procedure_response.status(), StatusCode::OK);

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

    for path in [
        "/api/query/search-entries",
        "/api/query/search-episodes",
        "/api/query/search-claims",
        "/api/query/search-procedures",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .method("POST")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", seeded.workspace_token))
                    .body(Body::from(
                        serde_json::json!({
                            "query": "rebuild",
                            "limit": 10,
                            "filter": { "workspace_root": "/workspace/http" }
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

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
        assert_eq!(response.status(), StatusCode::OK);
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
fn mcp_exposes_canonical_resources_and_compat_aliases() {
    let seeded = seed_app();
    let workspace_id = workspace_stable_id("/workspace/http");

    for uri in [
        format!("axiom://sessions/{}", seeded.session_id),
        format!("axiom://entries/{}", seeded.entry_id),
        format!("axiom://artifacts/{}", seeded.artifact_id),
        format!("axiom://anchors/{}", seeded.anchor_id),
        format!("axiom://episodes/{}", seeded.episode_id),
        format!("axiom://cases/{}", seeded.episode_id),
        format!("axiom://threads/{}", seeded.session_id),
        format!("axiom://runbooks/{}", seeded.episode_id),
        format!("axiom://tasks/{}", seeded.task_id),
    ] {
        let response = axiomsync::mcp::handle_request(
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

    let procedure_resource = axiomsync::mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": { "uri": format!("axiom://procedures/{}", seeded.procedure_id) }
        }),
        None,
    )
    .expect("procedure resource");
    assert!(procedure_resource.get("result").is_some());

    let resources = axiomsync::mcp::handle_request(
        &seeded.app,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"resources/list"}),
        Some(&workspace_id),
    )
    .expect("resources");
    let resource_uris = resources["result"]["resources"]
        .as_array()
        .expect("resources array")
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(resource_uris.iter().any(|uri| uri == "axiom://sessions/{id}"));
    assert!(resource_uris.iter().any(|uri| uri == "axiom://tasks/{id}"));

    let tools = axiomsync::mcp::handle_request(
        &seeded.app,
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/list"}),
        Some(&workspace_id),
    )
    .expect("tools");
    let tool_names = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(tool_names.iter().any(|name| name == "search_entries"));
    assert!(tool_names.iter().any(|name| name == "get_task"));

    let task_tool = axiomsync::mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": { "name": "get_task", "arguments": { "id": seeded.task_id } }
        }),
        Some(&workspace_id),
    )
    .expect("task tool");
    assert!(task_tool.get("result").is_some());

    let procedure_tool = axiomsync::mcp::handle_request(
        &seeded.app,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": { "name": "get_runbook", "arguments": { "id": seeded.episode_id } }
        }),
        Some(&workspace_id),
    )
    .expect("runbook tool");
    assert!(procedure_tool.get("result").is_some());
}
