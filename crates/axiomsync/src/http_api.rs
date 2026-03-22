use std::net::SocketAddr;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::{
    ConnectorBatchInput, EpisodeStatus, SearchEpisodesFilter, SearchEpisodesRequest,
};
use crate::error::{AxiomError, Result};
use crate::kernel::AxiomSync;
use crate::mcp;
use crate::web_ui;

#[derive(Clone)]
pub struct AppState {
    pub app: AxiomSync,
}

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/episodes/{id}", get(episode_page))
        .route("/connectors", get(connectors_page))
        .route("/health", get(health))
        .route("/ingest/{connector}", post(ingest))
        .route("/project", post(project))
        .route("/derive", post(derive))
        .route("/api/episodes", get(search))
        .route("/api/runbooks/{id}", get(runbook))
        .route("/api/threads/{id}", get(thread))
        .route("/api/evidence/{id}", get(evidence))
        .route("/mcp", post(mcp_http))
        .with_state(AppState { app })
}

pub fn connector_ingest_router(app: AxiomSync, connector: &str) -> Router {
    Router::new()
        .route("/health", get(connector_health))
        .route("/", post(connector_ingest))
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

async fn health(State(state): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "db_path": state.app.db_path(),
    })))
}

async fn index(headers: HeaderMap, State(state): State<AppState>) -> Result<Html<String>> {
    authorize_any(&state.app, &headers)?;
    Ok(Html(web_ui::index(&state.app.list_runbooks()?)))
}

async fn episode_page(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>> {
    let workspace_id = state.app.runbook_workspace_id(&id)?;
    authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Html(web_ui::episode(&state.app.get_runbook(&id)?)))
}

async fn connectors_page(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Html<String>> {
    authorize_any(&state.app, &headers)?;
    Ok(Html(web_ui::connectors(&state.app.connector_status()?)))
}

async fn ingest(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(connector): Path<String>,
    Json(mut batch): Json<ConnectorBatchInput>,
) -> Result<Json<Value>> {
    authorize_any(&state.app, &headers)?;
    for event in &mut batch.events {
        event.connector = connector.clone();
    }
    let plan = state.app.plan_ingest(&batch)?;
    let applied = state.app.apply_ingest(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

async fn project(headers: HeaderMap, State(state): State<AppState>) -> Result<Json<Value>> {
    authorize_any(&state.app, &headers)?;
    let plan = state.app.plan_projection()?;
    let applied = state.app.apply_projection(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

async fn derive(headers: HeaderMap, State(state): State<AppState>) -> Result<Json<Value>> {
    authorize_any(&state.app, &headers)?;
    let plan = state.app.plan_derivation()?;
    let applied = state.app.apply_derivation(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    connector: Option<String>,
    workspace_id: Option<String>,
    status: Option<String>,
}

fn default_limit() -> usize {
    10
}

async fn search(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>> {
    let workspace_id = authorize(&state.app, &headers, query.workspace_id.as_deref())?;
    let rows = state.app.search_episodes(SearchEpisodesRequest {
        query: query.query,
        limit: query.limit,
        filter: SearchEpisodesFilter {
            connector: query.connector,
            workspace_id,
            status: query
                .status
                .as_deref()
                .map(EpisodeStatus::parse)
                .transpose()?,
        },
    })?;
    Ok(Json(serde_json::to_value(rows)?))
}

async fn runbook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let runbook = state.app.get_runbook(&id)?;
    authorize(&state.app, &headers, runbook.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(runbook)?))
}

async fn thread(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let workspace_id = state.app.thread_workspace_id(&id)?;
    authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_thread(&id)?)?))
}

async fn evidence(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let workspace_id = state.app.evidence_workspace_id(&id)?;
    authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_evidence(&id)?)?))
}

async fn mcp_http(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<Value>,
) -> Result<Json<Value>> {
    let workspace_id = mcp::workspace_requirement(&state.app, &request)?.or_else(|| {
        request
            .get("params")
            .and_then(|params| params.get("workspace_id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    });
    let bound_workspace_id = authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(mcp::handle_request(
        &state.app,
        request,
        bound_workspace_id.as_deref(),
    )?))
}

#[derive(Clone)]
struct ConnectorIngestState {
    app: AxiomSync,
    connector: String,
}

async fn connector_ingest(
    State(state): State<ConnectorIngestState>,
    Json(value): Json<Value>,
) -> Result<Json<Value>> {
    let mut batch: ConnectorBatchInput = if value.get("events").is_some() {
        serde_json::from_value(value)?
    } else if value.is_array() {
        serde_json::from_value(json!({ "events": value }))?
    } else {
        serde_json::from_value(json!({ "events": [value] }))?
    };
    for event in &mut batch.events {
        event.connector = state.connector.clone();
    }
    let plan = state.app.plan_ingest(&batch)?;
    let applied = state.app.apply_ingest(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

async fn connector_health(State(state): State<ConnectorIngestState>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "connector": state.connector,
    })))
}

fn authorize(
    app: &AxiomSync,
    headers: &HeaderMap,
    workspace_id: Option<&str>,
) -> Result<Option<String>> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_workspace(token, workspace_id)
}

fn authorize_any(app: &AxiomSync, headers: &HeaderMap) -> Result<Option<String>> {
    authorize(app, headers, None)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
