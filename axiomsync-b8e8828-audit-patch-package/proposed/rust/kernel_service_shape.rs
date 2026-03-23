// compile-oriented skeleton, not build-verified

pub trait TransactionManager: Send + Sync {
    fn apply_ingest_tx(&self, plan: &IngestPlan) -> Result<serde_json::Value>;
    fn apply_source_cursor_tx(&self, plan: &SourceCursorUpsertPlan) -> Result<serde_json::Value>;
    fn apply_projection_tx(&self, plan: &ProjectionPlan) -> Result<serde_json::Value>;
    fn apply_derivation_tx(&self, plan: &DerivePlan) -> Result<serde_json::Value>;
    fn apply_replay_tx(&self, plan: &ReplayPlan) -> Result<serde_json::Value>;
    fn apply_purge_tx(&self, plan: &PurgePlan) -> Result<serde_json::Value>;
    fn apply_repair_tx(&self, plan: &RepairPlan) -> Result<serde_json::Value>;
}

#[derive(Clone)]
pub struct AxiomSync {
    repo: SharedRepositoryPort,
    auth: SharedAuthStorePort,
    llm: SharedLlmExtractionPort,
}

impl AxiomSync {
    pub fn plan_append_raw_events(&self, input: &ConnectorBatchInput) -> Result<IngestPlan> {
        let existing = self.repo.existing_raw_event_keys()?;
        plan_ingest(&existing, input)
    }

    pub fn apply_ingest_plan(&self, plan: &IngestPlan) -> Result<serde_json::Value> {
        self.repo.apply_ingest_tx(plan)
    }

    pub fn plan_upsert_source_cursor(
        &self,
        input: &SourceCursorInput,
    ) -> Result<SourceCursorUpsertPlan> {
        input.validate()?;
        Ok(SourceCursorUpsertPlan {
            row: SourceCursorRow {
                connector: input.connector.clone(),
                cursor_key: input.cursor_key.clone(),
                cursor_value: input.cursor_value.clone(),
                updated_at_ms: input.updated_at_ms,
            },
        })
    }

    pub fn apply_source_cursor_plan(
        &self,
        plan: &SourceCursorUpsertPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        self.repo.apply_source_cursor_tx(plan)
    }
}
