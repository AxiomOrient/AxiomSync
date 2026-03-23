use crate::domain::{
    AdminTokenPlan, AuthGrantRecord, AuthSnapshot, WorkspaceTokenPlan, stable_hash,
    workspace_stable_id,
};
use crate::error::Result;

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
