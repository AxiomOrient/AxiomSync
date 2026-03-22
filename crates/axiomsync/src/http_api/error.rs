use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::error::AxiomError;

pub(super) type HttpResult<T> = std::result::Result<T, HttpError>;

#[derive(Debug)]
pub(super) struct HttpError(pub(super) AxiomError);

impl From<AxiomError> for HttpError {
    fn from(value: AxiomError) -> Self {
        Self(value)
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(value: serde_json::Error) -> Self {
        Self(AxiomError::from(value))
    }
}

impl IntoResponse for HttpError {
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
