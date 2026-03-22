use std::net::SocketAddr;

use axum::Router;
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

#[derive(Clone)]
pub(crate) struct ConnectorIngestState {
    pub app: AxiomSync,
    pub connector: String,
}

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .route("/episodes/{id}", get(handlers::episode_page))
        .route("/connectors", get(handlers::connectors_page))
        .route("/health", get(handlers::health))
        .route("/ingest/{connector}", post(handlers::ingest))
        .route("/project", post(handlers::project))
        .route("/derive", post(handlers::derive))
        .route("/api/episodes", get(handlers::search))
        .route("/api/runbooks/{id}", get(handlers::runbook))
        .route("/api/threads/{id}", get(handlers::thread))
        .route("/api/evidence/{id}", get(handlers::evidence))
        .route("/mcp", post(handlers::mcp_http))
        .with_state(AppState { app })
}

pub fn connector_ingest_router(app: AxiomSync, connector: &str) -> Router {
    Router::new()
        .route("/health", get(handlers::connector_health))
        .route("/", post(handlers::connector_ingest))
        .with_state(ConnectorIngestState {
            app,
            connector: connector.to_string(),
        })
}

pub async fn serve(app: AxiomSync, addr: SocketAddr) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(app))
        .await
        .map_err(|error| AxiomError::Internal(format!("http server exited unexpectedly: {error}")))
}

pub async fn serve_connector_ingest(
    app: AxiomSync,
    addr: SocketAddr,
    connector: &str,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, connector_ingest_router(app, connector))
        .await
        .map_err(|error| {
            AxiomError::Internal(format!("connector server exited unexpectedly: {error}"))
        })
}
