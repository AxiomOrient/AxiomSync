use axum::{extract::State, http::StatusCode, Json};
use axiomsync_domain::model::RawEventBatch;
use serde::Serialize;

#[derive(Clone)]
pub struct AppState;

#[derive(Debug, Serialize)]
pub struct AppendRawEventsResponse {
    pub accepted: usize,
    pub rejected: usize,
    pub projection_required: bool,
}

pub async fn append_raw_events(
    State(_state): State<AppState>,
    Json(batch): Json<RawEventBatch>,
) -> Result<Json<AppendRawEventsResponse>, StatusCode> {
    let accepted = batch.events.len();
    Ok(Json(AppendRawEventsResponse {
        accepted,
        rejected: 0,
        projection_required: accepted > 0,
    }))
}
