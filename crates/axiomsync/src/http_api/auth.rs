use axum::http::HeaderMap;

use super::*;

pub(crate) fn authorize(
    app: &AxiomSync,
    headers: &HeaderMap,
    workspace_id: Option<&str>,
) -> Result<Option<String>> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_workspace(token, workspace_id)
}

pub(crate) fn authorize_any(app: &AxiomSync, headers: &HeaderMap) -> Result<Option<String>> {
    authorize(app, headers, None)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
