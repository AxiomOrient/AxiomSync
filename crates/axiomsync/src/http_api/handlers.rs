use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;
use crate::domain::{
    ConnectorBatchInput, EpisodeStatus, SearchEpisodesFilter, SearchEpisodesRequest,
};
use crate::http_api::error::HttpResult;
use crate::mcp;
use crate::web_ui;

#[derive(Debug, Deserialize)]
pub(super) struct SearchQuery {
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

pub(super) async fn health(State(state): State<AppState>) -> HttpResult<Json<Value>> {
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "db_path": state.app.db_path(),
    })))
}

pub(super) async fn index(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Html<String>> {
    auth::authorize_any(&state.app, &headers)?;
    Ok(Html(web_ui::index(&state.app.list_runbooks()?)))
}

pub(super) async fn episode_page(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Html<String>> {
    let workspace_id = state.app.runbook_workspace_id(&id)?;
    auth::authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Html(web_ui::episode(&state.app.get_runbook(&id)?)))
}

pub(super) async fn connectors_page(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Html<String>> {
    auth::authorize_any(&state.app, &headers)?;
    Ok(Html(web_ui::connectors(&state.app.connector_status()?)))
}

pub(super) async fn ingest(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(connector): Path<String>,
    Json(mut batch): Json<ConnectorBatchInput>,
) -> HttpResult<Json<Value>> {
    auth::authorize_any(&state.app, &headers)?;
    for event in &mut batch.events {
        event.connector = connector.clone();
    }
    let plan = state.app.plan_ingest(&batch)?;
    let applied = state.app.apply_ingest(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

pub(super) async fn project(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    auth::authorize_any(&state.app, &headers)?;
    let plan = state.app.plan_projection()?;
    let applied = state.app.apply_projection(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

pub(super) async fn derive(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    auth::authorize_any(&state.app, &headers)?;
    let plan = state.app.plan_derivation()?;
    let applied = state.app.apply_derivation(&plan)?;
    Ok(Json(json!({"plan": plan, "applied": applied})))
}

pub(super) async fn search(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> HttpResult<Json<Value>> {
    let workspace_id = auth::authorize(&state.app, &headers, query.workspace_id.as_deref())?;
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

pub(super) async fn runbook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let runbook = state.app.get_runbook(&id)?;
    auth::authorize(&state.app, &headers, runbook.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(runbook)?))
}

pub(super) async fn thread(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.thread_workspace_id(&id)?;
    auth::authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_thread(&id)?)?))
}

pub(super) async fn evidence(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.evidence_workspace_id(&id)?;
    auth::authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_evidence(&id)?)?))
}

pub(super) async fn mcp_http(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<Value>,
) -> HttpResult<Json<Value>> {
    let workspace_id = mcp::workspace_requirement(&state.app, &request)?.or_else(|| {
        request
            .get("params")
            .and_then(|params| params.get("workspace_id"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    });
    let bound_workspace_id = auth::authorize(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(mcp::handle_request(
        &state.app,
        request,
        bound_workspace_id.as_deref(),
    )?))
}

pub(super) async fn connector_ingest(
    State(state): State<ConnectorIngestState>,
    Json(value): Json<Value>,
) -> HttpResult<Json<Value>> {
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

pub(super) async fn connector_health(
    State(state): State<ConnectorIngestState>,
) -> HttpResult<Json<Value>> {
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "connector": state.connector,
    })))
}
