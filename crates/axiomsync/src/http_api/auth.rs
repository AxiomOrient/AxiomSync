use std::net::IpAddr;

use axum::http::HeaderMap;

use super::*;

pub(crate) fn authorize_workspace(
    app: &AxiomSync,
    headers: &HeaderMap,
    workspace_id: Option<&str>,
) -> Result<Option<String>> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_workspace(token, workspace_id)
}

pub(crate) fn authorize_admin(app: &AxiomSync, headers: &HeaderMap) -> Result<()> {
    let token = bearer_token(headers)
        .ok_or_else(|| AxiomError::PermissionDenied("missing bearer token".to_string()))?;
    app.authorize_admin(token)
}

pub(crate) fn reject_non_loopback(ip: IpAddr) -> Result<()> {
    if ip.is_loopback() {
        return Ok(());
    }
    Err(AxiomError::PermissionDenied(
        "sink routes require loopback source address".to_string(),
    ))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
