use super::*;

impl AxiomSync {
    pub fn build_append_batch(
        &self,
        request: &AppendRawEventsRequest,
    ) -> Result<ConnectorBatchInput> {
        crate::logic::append_request_batch(request)
    }

    pub fn plan_source_cursor_upsert(
        &self,
        request: &UpsertSourceCursorRequest,
    ) -> Result<SourceCursorUpsertPlan> {
        plan_source_cursor_upsert(request)
    }

    pub fn apply_source_cursor_upsert(&self, plan: &SourceCursorUpsertPlan) -> Result<Value> {
        self.repo.apply_source_cursor_upsert_tx(plan)
    }
}
