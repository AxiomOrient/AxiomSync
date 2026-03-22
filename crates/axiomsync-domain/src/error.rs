use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AxiomError>;

#[derive(Debug, Error)]
pub enum AxiomError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("llm is not configured: {0}")]
    LlmUnavailable(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

impl IntoResponse for AxiomError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AxiomError::Validation(_) => StatusCode::BAD_REQUEST,
            AxiomError::NotFound(_) => StatusCode::NOT_FOUND,
            AxiomError::PermissionDenied(_) => StatusCode::UNAUTHORIZED,
            AxiomError::Conflict(_) => StatusCode::CONFLICT,
            AxiomError::LlmUnavailable(_) => StatusCode::PRECONDITION_FAILED,
            AxiomError::Io(_)
            | AxiomError::Json(_)
            | AxiomError::Sqlite(_)
            | AxiomError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(serde_json::json!({"error": self.to_string()}))).into_response()
    }
}
