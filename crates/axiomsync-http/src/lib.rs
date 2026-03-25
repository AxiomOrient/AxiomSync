use std::net::{IpAddr, SocketAddr};

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use axiomsync_domain::{
    AppendRawEventsRequest, DerivePlan, ProjectionPlan, ReplayPlan, SearchCasesRequest,
    SourceCursorUpsertPlan, UpsertSourceCursorRequest,
};
use axiomsync_kernel::{AxiomError, AxiomSync, Result};
use axiomsync_mcp as mcp;

#[derive(Clone)]
pub struct AppState {
    pub app: AxiomSync,
}

#[derive(serde::Deserialize)]
struct RunsQuery {
    workspace_root: String,
}

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", get(index))
        .route("/sink/raw-events/plan", post(plan_append_raw_events))
        .route("/sink/raw-events/apply", post(apply_ingest_plan))
        .route("/sink/source-cursors/plan", post(plan_source_cursor_upsert))
        .route("/sink/source-cursors/apply", post(apply_source_cursor_plan))
        .route("/admin/projection/plan", post(plan_projection))
        .route("/admin/projection/apply", post(apply_projection))
        .route("/admin/derivations/plan", post(plan_derivations))
        .route("/admin/derivations/apply", post(apply_derivations))
        .route("/admin/replay/plan", post(plan_replay))
        .route("/admin/replay/apply", post(apply_replay))
        .route("/api/query/search-cases", post(search_cases))
        .route("/api/cases/{id}", get(get_case))
        .route("/api/threads/{id}", get(get_thread))
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
        "pending_projection_count": pending_projection_count,
        "pending_derived_count": pending_derived_count,
        "pending_index_count": pending_index_count,
    })))
}

async fn index(headers: HeaderMap, State(state): State<AppState>) -> HttpResult<Html<String>> {
    authorize_admin(&state.app, &headers)?;
    let cases = state.app.count_cases()?;
    let thread_count = state.app.count_sessions_by_kind("thread")?;
    let run_count = state.app.count_sessions_by_kind("run")?;
    let document_count = state.app.count_documents()?;
    Ok(Html(render_index_page(
        cases,
        thread_count,
        run_count,
        document_count,
    )))
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
    payload: std::result::Result<Json<axiomsync_domain::IngestPlan>, JsonRejection>,
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

async fn plan_projection(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(
        state.app.build_projection_plan()?,
    )?))
}

async fn apply_projection(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<ProjectionPlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_projection_plan(&plan)?))
}

async fn plan_derivations(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(
        state.app.build_derivation_plan()?,
    )?))
}

async fn apply_derivations(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<DerivePlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_derivation_plan(&plan)?))
}

async fn plan_replay(headers: HeaderMap, State(state): State<AppState>) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    Ok(Json(serde_json::to_value(state.app.build_replay_plan()?)?))
}

async fn apply_replay(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<ReplayPlan>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    authorize_admin(&state.app, &headers)?;
    let Json(plan) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    Ok(Json(state.app.apply_replay(&plan)?))
}

async fn search_cases(
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: std::result::Result<Json<SearchCasesRequest>, JsonRejection>,
) -> HttpResult<Json<Value>> {
    let Json(request) = payload.map_err(|error| AxiomError::Validation(error.body_text()))?;
    let workspace_root =
        request.filter.workspace_root.as_deref().ok_or_else(|| {
            AxiomError::Validation("workspace_root filter is required".to_string())
        })?;
    authorize_workspace_filter(&state.app, &headers, Some(workspace_root))?;
    Ok(Json(serde_json::to_value(
        state.app.search_cases(request)?,
    )?))
}

async fn get_case(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_resource(&state.app, &headers, state.app.case_workspace_id(&id)?)?;
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

async fn list_runs(
    headers: HeaderMap,
    Query(query): Query<RunsQuery>,
    State(state): State<AppState>,
) -> HttpResult<Json<Value>> {
    authorize_workspace_filter(&state.app, &headers, Some(query.workspace_root.as_str()))?;
    Ok(Json(serde_json::to_value(
        state.app.list_runs(Some(query.workspace_root.as_str()))?,
    )?))
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
    let id = mcp::rpc_id(&request);
    let parsed = match mcp::parse_request(&request) {
        Ok(parsed) => parsed,
        // Structural request errors use -32600 (Invalid Request), not -32602 (Invalid Params)
        Err(error) => return Ok(Json(mcp::request_parse_error_response(id, &error))),
    };
    let workspace_id = match mcp::workspace_requirement(&state.app, &parsed) {
        Ok(workspace_id) => workspace_id,
        Err(error) => return Ok(Json(mcp::error_response(parsed.id.clone(), &error))),
    };
    if let Some(required) = workspace_id.as_deref() {
        if let Err(error) =
            authorize_workspace_resource(&state.app, &headers, Some(required.to_string()))
        {
            return Ok(Json(mcp::error_response(parsed.id.clone(), &error)));
        }
    } else if let Err(error) = authorize_admin(&state.app, &headers) {
        return Ok(Json(mcp::error_response(parsed.id.clone(), &error)));
    }
    Ok(Json(
        match mcp::handle_parsed_request(&state.app, parsed, workspace_id.as_deref()) {
            Ok(response) => response,
            Err(error) => mcp::error_response(id, &error),
        },
    ))
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
    let workspace_id = workspace_root.map(axiomsync_domain::workspace_stable_id);
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

fn render_index_page(
    case_count: usize,
    thread_count: usize,
    run_count: usize,
    document_count: usize,
) -> String {
    let mut html = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>AxiomSync</title><style>body{font-family:ui-sans-serif,system-ui,sans-serif;margin:0;background:#f4f1ea;color:#1f2933}main{max-width:960px;margin:0 auto;padding:32px 20px 64px}section{background:#fff;border:1px solid #ddd7cc;border-radius:16px;padding:18px 20px;margin:14px 0}h1,h2{margin:0 0 12px}ul{padding-left:20px}a{color:#0f4c5c;text-decoration:none}code{font-family:ui-monospace,SFMono-Regular,monospace}</style></head><body><main>",
    );
    html.push_str("<h1>AxiomSync Knowledge Kernel</h1><p>Canonical read views for cases, threads, runs, documents, and evidence.</p>");
    html.push_str(&format!(
        "<section><h2>Cases</h2><p>{case_count} case records available.</p></section>"
    ));
    html.push_str(&format!(
        "<section><h2>Threads</h2><p>{thread_count} thread records available.</p></section>"
    ));
    html.push_str(&format!(
        "<section><h2>Runs</h2><p>{run_count} run records available.</p></section>"
    ));
    html.push_str(&format!(
        "<section><h2>Documents</h2><p>{document_count} document records available.</p></section>"
    ));
    html.push_str(
        "<section><h2>Evidence</h2><p>Evidence anchors are available through canonical case, thread, run, task, and document reads.</p></section>",
    );
    html.push_str("</main></body></html>");
    html
}
