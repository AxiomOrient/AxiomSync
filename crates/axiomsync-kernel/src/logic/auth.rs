use crate::domain::{
    AdminTokenPlan, AuthGrantRecord, AuthSnapshot, WorkspaceTokenPlan, stable_hash,
    workspace_stable_id,
};
use crate::error::{AxiomError, Result};

pub fn plan_workspace_token_grant(canonical_root: &str, token: &str) -> Result<WorkspaceTokenPlan> {
    Ok(WorkspaceTokenPlan {
        workspace_id: workspace_stable_id(canonical_root.trim()),
        token_sha256: stable_hash(&["workspace-token", token]),
    })
}

pub fn plan_admin_token_grant(token: &str) -> Result<AdminTokenPlan> {
    Ok(AdminTokenPlan {
        token_sha256: stable_hash(&["admin-token", token]),
    })
}

pub fn apply_workspace_token_plan(
    snapshot: &AuthSnapshot,
    plan: &WorkspaceTokenPlan,
) -> AuthSnapshot {
    let mut next = snapshot.clone();
    next.schema_version = crate::domain::KERNEL_SCHEMA_VERSION.to_string();
    next.grants
        .retain(|grant| grant.workspace_id != plan.workspace_id);
    next.grants.push(AuthGrantRecord {
        workspace_id: plan.workspace_id.clone(),
        token_sha256: plan.token_sha256.clone(),
    });
    next.grants
        .sort_by(|left, right| left.workspace_id.cmp(&right.workspace_id));
    next
}

pub fn apply_admin_token_plan(snapshot: &AuthSnapshot, plan: &AdminTokenPlan) -> AuthSnapshot {
    let mut next = snapshot.clone();
    next.schema_version = crate::domain::KERNEL_SCHEMA_VERSION.to_string();
    next.admin_tokens
        .retain(|token| token != &plan.token_sha256);
    next.admin_tokens.push(plan.token_sha256.clone());
    next.admin_tokens.sort();
    next.admin_tokens.dedup();
    next
}

pub fn authorize_workspace_token(
    snapshot: &AuthSnapshot,
    token: &str,
    workspace_id: Option<&str>,
) -> Result<Option<String>> {
    let expected = workspace_id
        .ok_or_else(|| AxiomError::PermissionDenied("workspace scope is required".to_string()))?;
    let token_sha256 = stable_hash(&["workspace-token", token]);
    let grant = snapshot
        .grants
        .iter()
        .find(|grant| grant.token_sha256 == token_sha256)
        .ok_or_else(|| {
            AxiomError::PermissionDenied("token does not grant workspace access".to_string())
        })?;
    if expected != grant.workspace_id {
        return Err(AxiomError::PermissionDenied(
            "token does not grant access to requested workspace".to_string(),
        ));
    }
    Ok(Some(grant.workspace_id.clone()))
}

pub fn authorize_admin_token(snapshot: &AuthSnapshot, token: &str) -> Result<()> {
    let token_sha256 = stable_hash(&["admin-token", token]);
    if snapshot.admin_tokens.contains(&token_sha256) {
        Ok(())
    } else {
        Err(AxiomError::PermissionDenied(
            "token does not grant admin access".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_authorization_is_pure_and_scope_checked() {
        let snapshot = AuthSnapshot {
            schema_version: crate::domain::KERNEL_SCHEMA_VERSION.to_string(),
            grants: vec![AuthGrantRecord {
                workspace_id: "ws_1".to_string(),
                token_sha256: stable_hash(&["workspace-token", "token-1"]),
            }],
            admin_tokens: Vec::new(),
        };

        assert_eq!(
            authorize_workspace_token(&snapshot, "token-1", Some("ws_1")).expect("authorized"),
            Some("ws_1".to_string())
        );
        assert!(authorize_workspace_token(&snapshot, "token-1", None).is_err());
        assert!(authorize_workspace_token(&snapshot, "token-1", Some("ws_2")).is_err());
        assert!(authorize_workspace_token(&snapshot, "missing", Some("ws_1")).is_err());
    }

    #[test]
    fn admin_authorization_is_pure() {
        let snapshot = AuthSnapshot {
            schema_version: crate::domain::KERNEL_SCHEMA_VERSION.to_string(),
            grants: Vec::new(),
            admin_tokens: vec![stable_hash(&["admin-token", "admin-1"])],
        };

        assert!(authorize_admin_token(&snapshot, "admin-1").is_ok());
        assert!(authorize_admin_token(&snapshot, "missing").is_err());
    }
}
