// compile-oriented skeleton, not build-verified

use axum::{Json, Router, routing::{get, post}};
use serde_json::{Value, json};

pub fn router(app: AxiomSync) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sink/raw-events/plan", post(plan_append_raw_events))
        .route("/sink/raw-events/apply", post(apply_ingest_plan))
        .route("/sink/source-cursors/plan", post(plan_upsert_source_cursor))
        .route("/sink/source-cursors/apply", post(apply_source_cursor_plan))
        .route("/api/cases", get(search_cases))
        .route("/api/cases/{id}", get(get_case))
        .route("/api/threads/{id}", get(get_thread))
        .route("/api/evidence/{id}", get(get_evidence))
        .route("/mcp", post(mcp_http))
        .with_state(AppState { app })
}

async fn health(axum::extract::State(state): axum::extract::State<AppState>) -> anyhow::Result<Json<Value>> {
    Ok(Json(json!({
        "status": "ok",
        "root": state.app.root(),
        "db_path": state.app.db_path(),
    })))
}

// handlers elided
