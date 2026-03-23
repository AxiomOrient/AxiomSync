use std::sync::Arc;

use axiomsync::domain::{
    AppendRawEventsRequest, CursorInput, EpisodeExtraction, RawEventInput,
    UpsertSourceCursorRequest, VerificationExtraction, VerificationKind, VerificationStatus,
};
use axiomsync::http_api::router;
use axiomsync::kernel::AxiomSync;
use axiomsync::llm::MockLlmClient;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, Request, StatusCode};
use rusqlite::Connection;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tempfile::tempdir;
use tower::ServiceExt;

fn app() -> AxiomSync {
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
    app
}

fn loopback_router(app: AxiomSync) -> axum::Router {
    router(app).layer(MockConnectInfo(SocketAddr::from((
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        4400,
    ))))
}

fn remote_router(app: AxiomSync) -> axum::Router {
    router(app).layer(MockConnectInfo(SocketAddr::from((
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)),
        4400,
    ))))
}

fn raw_request() -> AppendRawEventsRequest {
    AppendRawEventsRequest {
        request_id: Some("req-1".to_string()),
        events: vec![RawEventInput {
            source: "chatgpt".to_string(),
            native_schema_version: Some("chatgpt-selection-v1".to_string()),
            native_session_id: "/c/abc123".to_string(),
            native_event_id: Some("evt-1".to_string()),
            event_type: "selection_captured".to_string(),
            ts_ms: 1_710_000_000_000,
            payload: json!({
                "workspace_root": "https://chatgpt.com",
                "turn_id": "msg-1",
                "actor": "assistant",
                "text": "selected text"
            }),
        }],
    }
}

#[tokio::test]
async fn sink_http_health_uses_main_web_contract() {
    let router = loopback_router(app());
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(value["status"].as_str(), Some("ok"));
    assert!(value["db_path"].as_str().is_some(), "{value}");
}

#[tokio::test]
async fn removed_legacy_routes_fail_cleanly() {
    let router = loopback_router(app());

    for uri in [
        "/raw-events",
        "/source-cursors",
        "/preview-plan",
        "/connectors",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/ingest/chatgpt")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sink_http_append_raw_events_accepts_and_dedupes() {
    let app = app();
    let router = loopback_router(app.clone());
    let request = raw_request();

    let first_plan = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/plan")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first_plan.status(), StatusCode::OK);
    let first_body = axum::body::to_bytes(first_plan.into_body(), usize::MAX)
        .await
        .expect("body");
    let first_value: serde_json::Value = serde_json::from_slice(&first_body).expect("json");
    assert_eq!(first_value["adds"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        first_value["skipped_dedupe_keys"].as_array().map(Vec::len),
        Some(0)
    );

    let first_apply = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/apply")
                .header("content-type", "application/json")
                .body(Body::from(first_body.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first_apply.status(), StatusCode::OK);

    let second_plan = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/plan")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second_plan.status(), StatusCode::OK);
    let second_body = axum::body::to_bytes(second_plan.into_body(), usize::MAX)
        .await
        .expect("body");
    let second_value: serde_json::Value = serde_json::from_slice(&second_body).expect("json");
    assert_eq!(second_value["adds"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        second_value["skipped_dedupe_keys"].as_array().map(Vec::len),
        Some(1)
    );
}

#[tokio::test]
async fn sink_http_rejects_invalid_raw_event() {
    let router = loopback_router(app());
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/plan")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "events": [{
                            "source": "chatgpt",
                            "event_type": "selection_captured",
                            "ts_ms": 1,
                            "payload": {}
                        }]
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sink_http_preview_plan_is_non_mutating() {
    let app = app();
    let router = loopback_router(app.clone());
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/plan")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&raw_request()).expect("json"),
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
    assert_eq!(value["adds"].as_array().map(Vec::len), Some(1));

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let raw_count: i64 = conn
        .query_row("select count(*) from raw_event", [], |row| row.get(0))
        .expect("count");
    assert_eq!(raw_count, 0);
}

#[tokio::test]
async fn sink_http_upserts_source_cursor_without_events() {
    let app = app();
    let router = loopback_router(app.clone());
    let request = UpsertSourceCursorRequest {
        source: "codex".to_string(),
        cursor: CursorInput {
            cursor_key: "events".to_string(),
            cursor_value: "cursor-1".to_string(),
            updated_at_ms: 1_710_000_000_000,
        },
    };
    let plan = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/source-cursors/plan")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(plan.status(), StatusCode::OK);
    let plan_body = axum::body::to_bytes(plan.into_body(), usize::MAX)
        .await
        .expect("body");

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/source-cursors/apply")
                .header("content-type", "application/json")
                .body(Body::from(plan_body))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let conn = Connection::open(app.db_path()).expect("sqlite");
    let cursor_value: String = conn
        .query_row(
            "select cursor_value from source_cursor where connector = ?1 and cursor_key = ?2",
            ["codex", "events"],
            |row| row.get(0),
        )
        .expect("cursor");
    assert_eq!(cursor_value, "cursor-1");
}

#[tokio::test]
async fn sink_http_rejects_non_loopback_requests() {
    let router = remote_router(app());
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sink/raw-events/plan")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&raw_request()).expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
