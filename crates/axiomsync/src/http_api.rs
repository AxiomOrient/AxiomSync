use std::net::SocketAddr;

use axum::Router;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::routing::{get, post};

use crate::error::{AxiomError, Result};
use crate::kernel::AxiomSync;

mod auth;
mod error;
mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub app: AxiomSync,
}

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .route("/cases/{id}", get(handlers::case_page))
        .route("/episodes/{id}", get(handlers::episode_page))
        .route("/health", get(handlers::health))
        .route(
            "/sink/raw-events/plan",
            post(handlers::plan_append_raw_events),
        )
        .route("/sink/raw-events/apply", post(handlers::apply_ingest_plan))
        .route(
            "/sink/source-cursors/plan",
            post(handlers::plan_source_cursor_upsert),
        )
        .route(
            "/sink/source-cursors/apply",
            post(handlers::apply_source_cursor_plan),
        )
        .route("/project/rebuild/plan", post(handlers::plan_rebuild))
        .route("/project/rebuild/apply", post(handlers::apply_replay_plan))
        .route("/project/purge/plan", post(handlers::plan_purge))
        .route("/project/purge/apply", post(handlers::apply_purge_plan))
        .route("/derive/plan", post(handlers::plan_derive))
        .route("/derive/apply", post(handlers::apply_derive_plan))
        .route("/api/cases", get(handlers::search_cases))
        .route("/api/cases/{id}", get(handlers::case_detail))
        .route("/api/episodes", get(handlers::search))
        .route("/api/runbooks/{id}", get(handlers::runbook))
        .route("/api/threads/{id}", get(handlers::thread))
        .route("/api/runs", get(handlers::runs))
        .route("/api/runs/{id}", get(handlers::run))
        .route("/api/tasks/{id}", get(handlers::task))
        .route("/api/documents", get(handlers::documents))
        .route("/api/documents/{id}", get(handlers::document))
        .route("/api/evidence/{id}", get(handlers::evidence))
        .route("/mcp", post(handlers::mcp_http))
        .with_state(AppState { app })
}

pub fn connect_info_router(app: AxiomSync) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
    router(app).into_make_service_with_connect_info::<SocketAddr>()
}

pub async fn serve(app: AxiomSync, addr: SocketAddr) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, connect_info_router(app))
        .await
        .map_err(|error| AxiomError::Internal(format!("http server exited unexpectedly: {error}")))
}
