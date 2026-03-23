use std::path::{Path, PathBuf};
use std::sync::Arc;

use axiomsync_domain::domain::{
    AdminTokenPlan, AnchorRow, ArtifactRow, AuthSnapshot, ClaimEvidenceRow, ClaimRow, DerivePlan,
    DoctorReport, EntryRow, EpisodeRow, IngestPlan, IngressReceiptRow, InsightAnchorRow,
    InsightRow, ProcedureEvidenceRow, ProcedureRow, ProjectionPlan, SearchDocsRow, SearchHit,
    SessionRow, SourceCursorRow, SourceCursorUpsertPlan, VerificationRow, WorkspaceTokenPlan,
};
use axiomsync_domain::error::Result;
use serde_json::Value;

pub trait LlmExtractionPort: Send + Sync {}

pub type SharedLlmExtractionPort = Arc<dyn LlmExtractionPort>;

pub trait RepositoryPort: Send + Sync {
    fn root(&self) -> &Path;
    fn db_path(&self) -> &Path;
    fn init_report(&self) -> Result<Value>;
    fn existing_dedupe_keys(&self) -> Result<Vec<String>>;
    fn load_receipts(&self) -> Result<Vec<IngressReceiptRow>>;
    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>>;
    fn apply_ingest(&self, plan: &IngestPlan) -> Result<Value>;
    fn apply_source_cursor_upsert(&self, plan: &SourceCursorUpsertPlan) -> Result<Value>;
    fn replace_projection(&self, plan: &ProjectionPlan) -> Result<Value>;
    fn replace_derivation(&self, plan: &DerivePlan) -> Result<Value>;
    fn load_sessions(&self) -> Result<Vec<SessionRow>>;
    fn load_entries(&self) -> Result<Vec<EntryRow>>;
    fn load_artifacts(&self) -> Result<Vec<ArtifactRow>>;
    fn load_anchors(&self) -> Result<Vec<AnchorRow>>;
    fn load_episodes(&self) -> Result<Vec<EpisodeRow>>;
    fn load_insights(&self) -> Result<Vec<InsightRow>>;
    fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>>;
    fn load_verifications(&self) -> Result<Vec<VerificationRow>>;
    fn load_claims(&self) -> Result<Vec<ClaimRow>>;
    fn load_claim_evidence(&self) -> Result<Vec<ClaimEvidenceRow>>;
    fn load_procedures(&self) -> Result<Vec<ProcedureRow>>;
    fn load_procedure_evidence(&self) -> Result<Vec<ProcedureEvidenceRow>>;
    fn load_search_docs(&self) -> Result<Vec<SearchDocsRow>>;
    fn pending_counts(&self) -> Result<(usize, usize, usize)>;
    fn doctor_report(&self) -> Result<DoctorReport>;
}

pub type SharedRepositoryPort = Arc<dyn RepositoryPort>;

pub trait AuthStorePort: Send + Sync {
    fn root(&self) -> &Path;
    fn path(&self) -> PathBuf;
    fn read(&self) -> Result<AuthSnapshot>;
    fn write(&self, snapshot: &AuthSnapshot) -> Result<()>;
}

pub type SharedAuthStorePort = Arc<dyn AuthStorePort>;

pub trait McpResourcePort: Send + Sync {
    fn mcp_resources(&self) -> Result<Value>;
    fn mcp_roots(&self, bound_workspace_id: Option<&str>) -> Result<Value>;
    fn read_mcp_resource(&self, uri: &str) -> Result<Value>;
    fn resource_workspace_requirement(&self, uri: &str) -> Result<Option<String>>;
}

pub trait McpToolPort: Send + Sync {
    fn mcp_tools(&self) -> Result<Value>;
    fn call_mcp_tool(
        &self,
        name: &str,
        arguments: &Value,
        bound_workspace_id: Option<&str>,
    ) -> Result<Value>;
    fn tool_workspace_requirement(&self, name: &str, arguments: &Value) -> Result<Option<String>>;
}

pub fn workspace_token_plan(canonical_root: &str, token: &str) -> WorkspaceTokenPlan {
    WorkspaceTokenPlan {
        workspace_id: axiomsync_domain::domain::workspace_stable_id(canonical_root),
        token_sha256: axiomsync_domain::domain::stable_hash(&["workspace-token", token]),
    }
}

pub fn admin_token_plan(token: &str) -> AdminTokenPlan {
    AdminTokenPlan {
        token_sha256: axiomsync_domain::domain::stable_hash(&["admin-token", token]),
    }
}

pub fn filter_hits(hits: Vec<SearchHit>, limit: usize) -> Vec<SearchHit> {
    if limit == 0 {
        hits
    } else {
        hits.into_iter().take(limit).collect()
    }
}
