use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::domain::{
    AppendRawEventsRequest, ConnectorBatchInput, DerivationContext, DerivationEnrichment,
    DerivationInputs, DerivePlan, DoctorReport, EvidenceView, ExistingRawEventKey, IngestPlan,
    ProjectionPlan, PurgePlan, RawEventRow, RepairPlan, ReplayPlan, RunbookRecord,
    SearchCommandsResult, SearchEpisodesRequest, SearchEpisodesResult, SourceCursorRow,
    SourceCursorUpsertPlan, ThreadView, UpsertSourceCursorRequest, WorkspaceTokenPlan,
};
use crate::error::{AxiomError, Result};
use crate::logic::{
    EpisodeSearchRows, apply_workspace_token_plan, merge_verification_extractions,
    parse_verification_transcript, plan_derivation, plan_derivation_contexts, plan_ingest,
    plan_projection, plan_purge, plan_repair, plan_replay, plan_source_cursor_upsert,
    plan_workspace_token_grant, search_command_results, search_episode_results,
};
use crate::ports::{
    McpResourcePort, McpToolPort, SharedAuthStorePort, SharedLlmExtractionPort,
    SharedRepositoryPort,
};

mod auth;
mod mcp;
mod planning;
mod query;
mod sink;

#[derive(Clone)]
pub struct AxiomSync {
    repo: SharedRepositoryPort,
    auth: SharedAuthStorePort,
    llm: SharedLlmExtractionPort,
}

impl std::fmt::Debug for AxiomSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxiomSync")
            .field("root", &self.root())
            .finish_non_exhaustive()
    }
}

impl AxiomSync {
    pub fn new(
        repo: SharedRepositoryPort,
        auth: SharedAuthStorePort,
        llm: SharedLlmExtractionPort,
    ) -> Self {
        Self { repo, auth, llm }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        self.repo.root()
    }

    #[must_use]
    pub fn db_path(&self) -> &Path {
        self.repo.db_path()
    }

    #[must_use]
    pub fn auth_path(&self) -> PathBuf {
        self.auth.path()
    }

    pub fn init(&self) -> Result<Value> {
        let mut report = self.repo.init_report()?;
        if let Some(object) = report.as_object_mut() {
            object.insert("auth_path".to_string(), serde_json::json!(self.auth_path()));
        }
        Ok(report)
    }
}
