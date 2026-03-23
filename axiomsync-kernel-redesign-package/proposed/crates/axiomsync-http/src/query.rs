use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct QueryState;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub session_kind: Option<String>,
    pub connector: Option<String>,
    pub workspace_root: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
}

pub async fn search_entries(
    State(_state): State<QueryState>,
    Json(_req): Json<SearchRequest>,
) -> Json<SearchResponse> {
    Json(SearchResponse { hits: vec![] })
}
