use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::{
    AuthSnapshot, ConvItemRow, ConvSessionRow, ConvTurnRow, DerivePlan, DoctorReport,
    DocumentRecordRow, DocumentView, EpisodeConnectorRow, EpisodeEvidenceSearchRow,
    EpisodeExtraction, EpisodeRow, EvidenceAnchorRow, EvidenceView, ExecutionApprovalRow,
    ExecutionCheckRow, ExecutionEventRow, ExecutionRunRow, ExecutionTaskRow, ExistingRawEventKey,
    ImportJournalRow, IngestPlan, InsightAnchorRow, InsightRow, ProjectionPlan, PurgePlan,
    RawEventRow, RepairPlan, ReplayPlan, RunView, SearchCommandCandidateRow, SearchDocRedactedRow,
    SearchEpisodeFtsRow, SourceCursorRow, SourceCursorUpsertPlan, TaskView, ThreadView,
    VerificationExtraction, VerificationRow, WorkspaceRow,
};
use crate::error::Result;
use serde_json::Value;

pub trait LlmExtractionPort: Send + Sync {
    fn extract_episode(&self, transcript: &str) -> Result<EpisodeExtraction>;
    fn synthesize_verifications(&self, transcript: &str) -> Result<Vec<VerificationExtraction>>;
}

pub type SharedLlmExtractionPort = Arc<dyn LlmExtractionPort>;

pub trait ReadRepository: Send + Sync {
    fn root(&self) -> &Path;
    fn db_path(&self) -> &Path;
    fn init_report(&self) -> Result<Value>;
    fn existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>>;
    fn load_raw_events(&self) -> Result<Vec<RawEventRow>>;
    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>>;
    fn load_import_journal(&self) -> Result<Vec<ImportJournalRow>>;
    fn load_workspaces(&self) -> Result<Vec<WorkspaceRow>>;
    fn load_sessions(&self) -> Result<Vec<ConvSessionRow>>;
    fn load_turns(&self) -> Result<Vec<ConvTurnRow>>;
    fn load_items(&self) -> Result<Vec<ConvItemRow>>;
    fn load_evidence_anchors(&self) -> Result<Vec<EvidenceAnchorRow>>;
    fn load_execution_runs(&self) -> Result<Vec<ExecutionRunRow>>;
    fn load_execution_tasks(&self) -> Result<Vec<ExecutionTaskRow>>;
    fn load_execution_checks(&self) -> Result<Vec<ExecutionCheckRow>>;
    fn load_execution_approvals(&self) -> Result<Vec<ExecutionApprovalRow>>;
    fn load_execution_events(&self) -> Result<Vec<ExecutionEventRow>>;
    fn load_document_records(&self) -> Result<Vec<DocumentRecordRow>>;
    fn load_episodes(&self) -> Result<Vec<EpisodeRow>>;
    fn load_insights(&self) -> Result<Vec<InsightRow>>;
    fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>>;
    fn load_verifications(&self) -> Result<Vec<VerificationRow>>;
    fn load_search_docs_redacted(&self) -> Result<Vec<SearchDocRedactedRow>>;
    fn get_thread(&self, session_id: &str) -> Result<ThreadView>;
    fn get_run(&self, run_id: &str) -> Result<RunView>;
    fn get_task(&self, task_id: &str) -> Result<TaskView>;
    fn get_document(&self, document_id: &str) -> Result<DocumentView>;
    fn get_evidence(&self, evidence_id: &str) -> Result<EvidenceView>;
    fn load_episode_connectors(&self) -> Result<Vec<EpisodeConnectorRow>>;
    fn load_episode_search_fts_rows(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchEpisodeFtsRow>>;
    fn load_episode_evidence_search_rows(&self) -> Result<Vec<EpisodeEvidenceSearchRow>>;
    fn load_command_search_candidates(&self) -> Result<Vec<SearchCommandCandidateRow>>;
    fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>>;
    fn thread_workspace_id(&self, thread_id: &str) -> Result<Option<String>>;
    fn run_workspace_id(&self, run_id: &str) -> Result<Option<String>>;
    fn task_workspace_id(&self, task_id: &str) -> Result<Option<String>>;
    fn document_workspace_id(&self, document_id: &str) -> Result<Option<String>>;
    fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>>;
    fn doctor_report(&self) -> Result<DoctorReport>;
}

pub trait WriteRepository: Send + Sync {
    fn delete_raw_events(&self, stable_ids: &[String]) -> Result<usize>;
    fn delete_source_cursors_for_connector(&self, connector: &str) -> Result<usize>;
    fn delete_import_journal_for_connector(&self, connector: &str) -> Result<usize>;
    fn clear_derived_state(&self) -> Result<()>;
}

pub trait TransactionManager: Send + Sync {
    fn apply_ingest_tx(&self, plan: &IngestPlan) -> Result<Value>;
    fn apply_source_cursor_upsert_tx(&self, plan: &SourceCursorUpsertPlan) -> Result<Value>;
    fn apply_projection_tx(&self, plan: &ProjectionPlan) -> Result<Value>;
    fn apply_derivation_tx(&self, plan: &DerivePlan) -> Result<Value>;
    fn apply_replay_tx(&self, plan: &ReplayPlan) -> Result<Value>;
    fn apply_purge_tx(&self, plan: &PurgePlan) -> Result<Value>;
    fn apply_repair_tx(&self, plan: &RepairPlan) -> Result<Value>;
}

pub trait RepositoryPort: ReadRepository + WriteRepository + TransactionManager {}

impl<T> RepositoryPort for T where T: ReadRepository + WriteRepository + TransactionManager + ?Sized {}

pub type SharedRepositoryPort = Arc<dyn RepositoryPort>;

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

pub trait AuthStorePort: Send + Sync {
    fn root(&self) -> &Path;
    fn path(&self) -> PathBuf;
    fn read(&self) -> Result<AuthSnapshot>;
    fn write(&self, snapshot: &AuthSnapshot) -> Result<()>;
}

pub type SharedAuthStorePort = Arc<dyn AuthStorePort>;
