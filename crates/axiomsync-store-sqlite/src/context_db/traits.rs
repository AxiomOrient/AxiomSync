use super::*;

impl ReadRepository for ContextDb {
    fn root(&self) -> &Path {
        ContextDb::root(self)
    }

    fn db_path(&self) -> &Path {
        ContextDb::db_path(self)
    }

    fn init_report(&self) -> Result<serde_json::Value> {
        ContextDb::init_report(self)
    }

    fn existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>> {
        ContextDb::existing_raw_event_keys(self)
    }

    fn load_raw_events(&self) -> Result<Vec<RawEventRow>> {
        ContextDb::load_raw_events(self)
    }

    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        ContextDb::load_source_cursors(self)
    }

    fn load_import_journal(&self) -> Result<Vec<ImportJournalRow>> {
        ContextDb::load_import_journal(self)
    }

    fn load_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        ContextDb::load_workspaces(self)
    }

    fn load_sessions(&self) -> Result<Vec<ConvSessionRow>> {
        ContextDb::load_sessions(self)
    }

    fn load_turns(&self) -> Result<Vec<ConvTurnRow>> {
        ContextDb::load_turns(self)
    }

    fn load_items(&self) -> Result<Vec<ConvItemRow>> {
        ContextDb::load_items(self)
    }

    fn load_evidence_anchors(&self) -> Result<Vec<EvidenceAnchorRow>> {
        ContextDb::load_evidence_anchors(self)
    }

    fn load_episodes(&self) -> Result<Vec<EpisodeRow>> {
        ContextDb::load_episodes(self)
    }

    fn load_insights(&self) -> Result<Vec<InsightRow>> {
        ContextDb::load_insights(self)
    }

    fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>> {
        ContextDb::load_insight_anchors(self)
    }

    fn load_verifications(&self) -> Result<Vec<VerificationRow>> {
        ContextDb::load_verifications(self)
    }

    fn load_search_docs_redacted(&self) -> Result<Vec<SearchDocRedactedRow>> {
        ContextDb::load_search_docs_redacted(self)
    }

    fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        ContextDb::get_thread(self, session_id)
    }

    fn get_evidence(&self, evidence_id: &str) -> Result<crate::domain::EvidenceView> {
        ContextDb::get_evidence(self, evidence_id)
    }

    fn load_episode_connectors(&self) -> Result<Vec<EpisodeConnectorRow>> {
        ContextDb::load_episode_connectors(self)
    }

    fn load_episode_search_fts_rows(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchEpisodeFtsRow>> {
        ContextDb::load_episode_search_fts_rows(self, query, limit)
    }

    fn load_episode_evidence_search_rows(&self) -> Result<Vec<EpisodeEvidenceSearchRow>> {
        ContextDb::load_episode_evidence_search_rows(self)
    }

    fn load_command_search_candidates(&self) -> Result<Vec<SearchCommandCandidateRow>> {
        ContextDb::load_command_search_candidates(self)
    }

    fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        ContextDb::episode_workspace_id(self, episode_id)
    }

    fn thread_workspace_id(&self, thread_id: &str) -> Result<Option<String>> {
        ContextDb::thread_workspace_id(self, thread_id)
    }

    fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        ContextDb::evidence_workspace_id(self, evidence_id)
    }

    fn doctor_report(&self) -> Result<DoctorReport> {
        ContextDb::doctor_report(self)
    }
}

impl WriteRepository for ContextDb {
    fn delete_raw_events(&self, stable_ids: &[String]) -> Result<usize> {
        ContextDb::delete_raw_events(self, stable_ids)
    }

    fn delete_source_cursors_for_connector(&self, connector: &str) -> Result<usize> {
        ContextDb::delete_source_cursors_for_connector(self, connector)
    }

    fn delete_import_journal_for_connector(&self, connector: &str) -> Result<usize> {
        ContextDb::delete_import_journal_for_connector(self, connector)
    }

    fn clear_derived_state(&self) -> Result<()> {
        ContextDb::clear_derived_state(self)
    }
}

impl TransactionManager for ContextDb {
    fn apply_ingest_tx(&self, plan: &IngestPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_ingest_in_tx(tx, plan))
    }

    fn apply_projection_tx(&self, plan: &ProjectionPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_projection_in_tx(tx, plan))
    }

    fn apply_derivation_tx(&self, plan: &DerivePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_derivation_in_tx(tx, plan))
    }

    fn apply_replay_tx(&self, plan: &ReplayPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_replay_in_tx(tx, plan))
    }

    fn apply_purge_tx(&self, plan: &PurgePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_purge_in_tx(tx, plan))
    }

    fn apply_repair_tx(&self, plan: &RepairPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_repair_in_tx(tx, plan))
    }
}
