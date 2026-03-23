use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;
use crate::domain::{
    AppendRawEventsRequest, DerivePlan, EpisodeStatus, IngestPlan, PurgePlan, ReplayPlan,
    SearchCasesFilter, SearchCasesRequest, SearchEpisodesFilter, SearchEpisodesRequest,
    SourceCursorUpsertPlan, UpsertSourceCursorRequest,
};
use crate::http_api::error::HttpResult;
use crate::mcp;
use crate::web_ui;

#[derive(Debug, Deserialize)]
pub(super) struct SearchQuery {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(alias = "connector", alias = "producer")]
    source: Option<String>,
    workspace_id: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RunsQuery {
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PurgeQuery {
    #[serde(alias = "connector")]
    source: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DocumentsQuery {
    workspace_id: Option<String>,
    kind: Option<String>,
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
    auth::authorize_admin(&state.app, &headers)?;
    Ok(Html(web_ui::index(&state.app.list_cases()?)))
}

pub(super) async fn case_page(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Html<String>> {
    let workspace_id = state.app.case_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Html(web_ui::case_page(&state.app.get_case(&id)?)))
}

pub(super) async fn episode_page(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Html<String>> {
    let workspace_id = state.app.case_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Html(web_ui::case_page(&state.app.get_case(&id)?)))
}

pub(super) async fn plan_rebuild(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(crate::build_replay_plan(
        &state.app,
    )?)?))
}

pub(super) async fn apply_replay_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<ReplayPlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_replay(&plan)?))
}

pub(super) async fn plan_purge(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<PurgeQuery>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    let plan = crate::build_purge_plan(
        &state.app,
        query.source.as_deref(),
        query.workspace_id.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(plan)?))
}

pub(super) async fn apply_purge_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<PurgePlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_purge(&plan)?))
}

pub(super) async fn plan_derive(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(crate::build_derivation_plan(
        &state.app,
    )?)?))
}

pub(super) async fn apply_derive_plan(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<DerivePlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    auth::authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_derivation(&plan)?))
}

pub(super) async fn search(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> HttpResult<Json<Value>> {
    let workspace_id =
        auth::authorize_workspace(&state.app, &headers, query.workspace_id.as_deref())?;
    let rows = state.app.search_episodes(SearchEpisodesRequest {
        query: query.query,
        limit: query.limit,
        filter: SearchEpisodesFilter {
            source: query.source,
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

pub(super) async fn search_cases(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> HttpResult<Json<Value>> {
    let workspace_id =
        auth::authorize_workspace(&state.app, &headers, query.workspace_id.as_deref())?;
    let rows = state.app.search_cases(SearchCasesRequest {
        query: query.query,
        limit: query.limit,
        filter: SearchCasesFilter {
            producer: query.source,
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

pub(super) async fn case_detail(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let case_record = state.app.get_case(&id)?;
    auth::authorize_workspace(&state.app, &headers, case_record.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(case_record)?))
}

pub(super) async fn runbook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let runbook = state.app.get_runbook(&id)?;
    auth::authorize_workspace(&state.app, &headers, runbook.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(runbook)?))
}

pub(super) async fn thread(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.thread_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_thread(&id)?)?))
}

pub(super) async fn runs(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<RunsQuery>,
) -> HttpResult<Json<Value>> {
    let workspace_id =
        auth::authorize_workspace(&state.app, &headers, query.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(
        state.app.list_runs(workspace_id.as_deref())?,
    )?))
}

pub(super) async fn run(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.run_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_run(&id)?)?))
}

pub(super) async fn task(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.task_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_task(&id)?)?))
}

pub(super) async fn documents(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<DocumentsQuery>,
) -> HttpResult<Json<Value>> {
    let workspace_id =
        auth::authorize_workspace(&state.app, &headers, query.workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.list_documents(
        workspace_id.as_deref(),
        query.kind.as_deref(),
    )?)?))
}

pub(super) async fn document(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.document_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(serde_json::to_value(state.app.get_document(&id)?)?))
}

pub(super) async fn evidence(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let workspace_id = state.app.evidence_workspace_id(&id)?;
    auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
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
    let bound_workspace_id =
        auth::authorize_workspace(&state.app, &headers, workspace_id.as_deref())?;
    Ok(Json(mcp::handle_request(
        &state.app,
        request,
        bound_workspace_id.as_deref(),
    )?))
}

type SinkHttpResult<T> = std::result::Result<T, SinkHttpError>;

#[derive(Debug)]
pub(super) struct SinkHttpError(AxiomError);

impl From<AxiomError> for SinkHttpError {
    fn from(value: AxiomError) -> Self {
        Self(value)
    }
}

impl From<serde_json::Error> for SinkHttpError {
    fn from(value: serde_json::Error) -> Self {
        Self(AxiomError::from(value))
    }
}

impl IntoResponse for SinkHttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            AxiomError::Validation(_) => StatusCode::BAD_REQUEST,
            AxiomError::NotFound(_) => StatusCode::NOT_FOUND,
            AxiomError::PermissionDenied(_) => StatusCode::UNAUTHORIZED,
            AxiomError::Conflict(_) => StatusCode::CONFLICT,
            AxiomError::LlmUnavailable(_) => StatusCode::PRECONDITION_FAILED,
            AxiomError::Io(_) | AxiomError::Json(_) | AxiomError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (
            status,
            Json(serde_json::json!({"error": self.0.to_string()})),
        )
            .into_response()
    }
}

pub(super) async fn plan_append_raw_events(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<AppendRawEventsRequest>, JsonRejection>,
) -> SinkHttpResult<Json<Value>> {
    auth::reject_non_loopback(addr.ip())?;
    let Json(request) = payload.map_err(json_rejection)?;
    let batch = state.app.build_append_batch(&request)?;
    let existing = state.app.load_existing_raw_event_keys()?;
    let plan = state.app.plan_ingest(&existing, &batch)?;
    Ok(Json(serde_json::to_value(plan)?))
}

pub(super) async fn apply_ingest_plan(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<IngestPlan>, JsonRejection>,
) -> SinkHttpResult<Json<Value>> {
    auth::reject_non_loopback(addr.ip())?;
    let Json(plan) = payload.map_err(json_rejection)?;
    Ok(Json(state.app.apply_ingest(&plan)?))
}

pub(super) async fn plan_source_cursor_upsert(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<UpsertSourceCursorRequest>, JsonRejection>,
) -> SinkHttpResult<Json<Value>> {
    auth::reject_non_loopback(addr.ip())?;
    let Json(request) = payload.map_err(json_rejection)?;
    let plan = state.app.plan_source_cursor_upsert(&request)?;
    Ok(Json(serde_json::to_value(plan)?))
}

pub(super) async fn apply_source_cursor_plan(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<SourceCursorUpsertPlan>, JsonRejection>,
) -> SinkHttpResult<Json<Value>> {
    auth::reject_non_loopback(addr.ip())?;
    let Json(plan) = payload.map_err(json_rejection)?;
    Ok(Json(state.app.apply_source_cursor_upsert(&plan)?))
}

fn json_rejection(error: JsonRejection) -> SinkHttpError {
    SinkHttpError(AxiomError::Validation(error.body_text()))
}
