use std::net::{IpAddr, SocketAddr};

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use axiomsync_domain::domain::{
    AppendRawEventsRequest, SearchClaimsRequest, SearchDocsRequest, SearchEntriesRequest,
    SearchEpisodesRequest, SearchInsightsRequest, SearchProceduresRequest, SourceCursorUpsertPlan,
    UpsertSourceCursorRequest,
};
use axiomsync_kernel::{AxiomError, AxiomSync, Result};
use axiomsync_mcp as mcp;

#[derive(Clone)]
pub struct AppState {
    pub app: AxiomSync,
}

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sink/raw-events/plan", post(plan_append_raw_events))
        .route("/sink/raw-events/apply", post(apply_ingest_plan))
        .route("/sink/source-cursors/plan", post(plan_source_cursor_upsert))
        .route("/sink/source-cursors/apply", post(apply_source_cursor_plan))
        .route("/admin/rebuild/projection", post(rebuild_projection))
        .route("/admin/rebuild/derivations", post(rebuild_derivations))
        .route("/admin/rebuild/index", post(rebuild_index))
        .route("/api/sessions/{id}", get(get_session))
        .route("/api/entries/{id}", get(get_entry))
        .route("/api/artifacts/{id}", get(get_artifact))
        .route("/api/anchors/{id}", get(get_anchor))
        .route("/api/episodes/{id}", get(get_episode))
        .route("/api/claims/{id}", get(get_claim))
        .route("/api/procedures/{id}", get(get_procedure))
        .route("/api/query/search-entries", post(search_entries))
        .route("/api/query/search-episodes", post(search_episodes))
        .route("/api/query/search-docs", post(search_docs))
        .route("/api/query/search-insights", post(search_insights))
        .route("/api/query/search-claims", post(search_claims))
        .route("/api/query/search-procedures", post(search_procedures))
        .route("/api/query/find-fix", post(find_fix))
        .route("/api/query/find-decision", post(find_decision))
        .route("/api/query/find-runbook", post(find_runbook))
        .route("/api/query/evidence-bundle", post(get_evidence_bundle))
        .route("/api/cases/{id}", get(get_case))
        .route("/api/threads/{id}", get(get_thread))
        .route("/api/runbooks/{id}", get(get_runbook))
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/documents/{id}", get(get_document))
        .route("/api/evidence/{id}", get(get_evidence))
        .route("/mcp", post(mcp_http))
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

async fn health(State(state): State<AppState>) -> HttpResult<Json<Value>> {
    let (pending_projection_count, pending_derived_count, pending_index_count) =
        state.app.pending_counts()?;
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "db_path": state.app.db_path(),
        "auth_path": state.app.auth_path(),
        "pending_projection_count": pending_projection_count,
        "pending_derived_count": pending_derived_count,
        "pending_index_count": pending_index_count,
    })))
}

async fn plan_append_raw_events(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<AppendRawEventsRequest>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    reject_non_loopback(addr.ip())?;
    let Json(request) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(serde_json::to_value(
        state.app.plan_append_raw_events(request)?,
    )?))
}

async fn apply_ingest_plan(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<axiomsync_domain::domain::IngestPlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    reject_non_loopback(addr.ip())?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_ingest_plan(&plan)?))
}

async fn plan_source_cursor_upsert(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<UpsertSourceCursorRequest>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    reject_non_loopback(addr.ip())?;
    let Json(request) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(serde_json::to_value(
        state.app.plan_source_cursor_upsert(request)?,
    )?))
}

async fn apply_source_cursor_plan(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    payload: std::result::Result<Json<SourceCursorUpsertPlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    reject_non_loopback(addr.ip())?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_source_cursor_plan(&plan)?))
}

async fn rebuild_projection(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let plan = state.app.build_projection_plan()?;
    Ok(Json(state.app.apply_projection_plan(&plan)?))
}

async fn rebuild_derivations(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let plan = state.app.build_derivation_plan()?;
    Ok(Json(state.app.apply_derivation_plan(&plan)?))
}

async fn rebuild_index(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let plan = state.app.build_replay_plan()?;
    Ok(Json(state.app.apply_replay(&plan)?))
}

async fn get_session(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.session_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_session(&id)?)?))
}

async fn get_entry(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let entry = state.app.get_entry(&id)?;
    authorize_workspace_resource(
        &state.app,
        &headers,
        state.app.session_workspace_id(&entry.session_id)?,
    )?;
    Ok(Json(serde_json::to_value(entry)?))
}

async fn get_artifact(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.artifact_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_artifact(&id)?)?))
}

async fn get_anchor(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.anchor_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_anchor(&id)?)?))
}

async fn get_episode(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.episode_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_episode(&id)?)?))
}

async fn get_claim(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    let claim = state.app.get_claim(&id)?;
    let workspace = match claim.episode_id.as_deref() {
        Some(episode_id) => state.app.episode_workspace_id(episode_id)?,
        None => None,
    };
    authorize_workspace_resource(&state.app, &headers, workspace)?;
    Ok(Json(serde_json::to_value(claim)?))
}

async fn get_procedure(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(state.app.get_procedure(&id)?)?))
}

async fn search_entries(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchEntriesRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_entries(payload.0)?,
    )?))
}

async fn search_episodes(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchEpisodesRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_episodes(payload.0)?,
    )?))
}

async fn search_claims(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchClaimsRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_claims(payload.0)?,
    )?))
}

async fn search_insights(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchInsightsRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_insights(payload.0)?,
    )?))
}

async fn search_docs(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchDocsRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_docs(payload.0)?,
    )?))
}

async fn search_procedures(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchProceduresRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.search_procedures(payload.0)?,
    )?))
}

async fn find_fix(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchInsightsRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(state.app.find_fix(payload.0)?)?))
}

async fn find_decision(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchInsightsRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.find_decision(payload.0)?,
    )?))
}

async fn find_runbook(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<SearchProceduresRequest>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(
        &state.app,
        &headers,
        payload.filter.workspace_root.as_deref(),
    )?;
    Ok(Json(serde_json::to_value(
        state.app.find_runbook(payload.0)?,
    )?))
}

#[derive(serde::Deserialize)]
struct EvidenceBundleRequest {
    subject_kind: String,
    subject_id: String,
}

async fn get_evidence_bundle(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Json<EvidenceBundleRequest>,
) -> HttpResult<Json<Value>> {
    let workspace = match payload.subject_kind.as_str() {
        "episode" => state.app.episode_workspace_id(&payload.subject_id)?,
        "session" => state.app.session_workspace_id(&payload.subject_id)?,
        "insight" => state.app.insight_workspace_id(&payload.subject_id)?,
        "procedure" => state.app.procedure_workspace_id(&payload.subject_id)?,
        _ => None,
    };
    authorize_workspace_resource(&state.app, &headers, workspace)?;
    Ok(Json(serde_json::to_value(state.app.get_evidence_bundle(
        &payload.subject_kind,
        &payload.subject_id,
    )?)?))
}

async fn get_case(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.episode_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_case(&id)?)?))
}

async fn get_thread(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.session_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_thread(&id)?)?))
}

async fn get_runbook(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.episode_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_runbook(&id)?)?))
}

async fn list_runs(headers: HeaderMap, State(state): State<AppState>) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(state.app.list_runs(None)?)?))
}

async fn get_run(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.session_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_run(&id)?)?))
}

async fn get_task(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.task_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_task(&id)?)?))
}

async fn get_document(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.artifact_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_document(&id)?)?))
}

async fn get_evidence(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.anchor_workspace_id(&id)?)?;
    Ok(Json(serde_json::to_value(state.app.get_evidence(&id)?)?))
}

async fn mcp_http(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<Value>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    let Json(request) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    let workspace_id = mcp::workspace_requirement(&state.app, &request)?;
    if let Some(required) = workspace_id.as_deref() {
        authorize_workspace_resource(&state.app, &headers, Some(required.to_string()))?;
    } else {
        authorize_admin(&state.app, &headers)?;
    }
    Ok(Json(mcp::handle_request(
        &state.app,
        request,
        workspace_id.as_deref(),
    )?))
}

type HttpResult<T> = std::result::Result<T, HttpError>;

struct HttpError(AxiomError);

impl From<AxiomError> for HttpError {
    fn from(value: AxiomError) -> Self {
        Self(value)
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(value: serde_json::Error) -> Self {
        Self(AxiomError::Json(value))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            AxiomError::Validation(_) => axum::http::StatusCode::BAD_REQUEST,
            AxiomError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            AxiomError::Conflict(_) => axum::http::StatusCode::CONFLICT,
            AxiomError::PermissionDenied(_) => axum::http::StatusCode::FORBIDDEN,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn authorize_admin(app: &AxiomSync, headers: &HeaderMap) -> Result<()> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_admin(token)
}

fn authorize_workspace_filter(
    app: &AxiomSync,
    headers: &HeaderMap,
    workspace_root: Option<&str>,
) -> Result<Option<String>> {
    let workspace_id = workspace_root.map(axiomsync_domain::domain::workspace_stable_id);
    authorize_workspace_resource(app, headers, workspace_id)
}

fn authorize_workspace_resource(
    app: &AxiomSync,
    headers: &HeaderMap,
    workspace_id: Option<String>,
) -> Result<Option<String>> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_workspace(token, workspace_id.as_deref())
}

fn reject_non_loopback(ip: IpAddr) -> Result<()> {
    if ip.is_loopback() {
        Ok(())
    } else {
        Err(AxiomError::PermissionDenied(
            "sink routes require loopback source address".to_string(),
        ))
    }
}

#[allow(dead_code)]
fn parse_json<T: DeserializeOwned>(value: Value) -> Result<T> {
    Ok(serde_json::from_value(value)?)
}
