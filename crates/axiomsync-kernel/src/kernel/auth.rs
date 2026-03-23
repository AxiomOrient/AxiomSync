use super::*;
use crate::domain::stable_hash;

impl AxiomSync {
    pub fn plan_admin_token_grant(&self, token: &str) -> Result<crate::domain::AdminTokenPlan> {
        crate::logic::plan_admin_token_grant(token)
    }

    pub fn apply_admin_token_grant(&self, plan: &crate::domain::AdminTokenPlan) -> Result<Value> {
        let snapshot = self.auth.read()?;
        let next = crate::logic::apply_admin_token_plan(&snapshot, plan);
        self.auth.write(&next)?;
        Ok(serde_json::json!({
            "admin_token_sha256": plan.token_sha256,
        }))
    }

    pub fn plan_workspace_token_grant(
        &self,
        canonical_root: &str,
        token: &str,
    ) -> Result<WorkspaceTokenPlan> {
        plan_workspace_token_grant(canonical_root, token)
    }

    pub fn apply_workspace_token_grant(&self, plan: &WorkspaceTokenPlan) -> Result<Value> {
        let snapshot = self.auth.read()?;
        let next = apply_workspace_token_plan(&snapshot, plan);
        self.auth.write(&next)?;
        Ok(serde_json::json!({
            "workspace_id": plan.workspace_id,
        }))
    }

    pub fn authorize_workspace(
        &self,
        token: &str,
        workspace_id: Option<&str>,
    ) -> Result<Option<String>> {
        let snapshot = self.auth.read()?;
        let token_sha = stable_hash(&[token]);
        let mut matched = snapshot
            .grants
            .iter()
            .filter(|entry| entry.token_sha256 == token_sha)
            .map(|entry| entry.workspace_id.clone())
            .collect::<Vec<_>>();
        matched.sort();
        matched.dedup();
        if matched.is_empty() {
            return Err(AxiomError::PermissionDenied(
                "invalid bearer token".to_string(),
            ));
        }
        if let Some(target) = workspace_id {
            if matched.iter().any(|candidate| candidate == target) {
                return Ok(Some(target.to_string()));
            }
            return Err(AxiomError::PermissionDenied(
                "token does not grant access to requested workspace".to_string(),
            ));
        }
        Ok(matched.into_iter().next())
    }

    pub fn authorize_admin(&self, token: &str) -> Result<()> {
        let snapshot = self.auth.read()?;
        let token_sha = stable_hash(&[token]);
        if snapshot
            .admin_tokens
            .iter()
            .any(|entry| entry == &token_sha)
        {
            return Ok(());
        }
        Err(AxiomError::PermissionDenied(
            "invalid admin bearer token".to_string(),
        ))
    }
}
