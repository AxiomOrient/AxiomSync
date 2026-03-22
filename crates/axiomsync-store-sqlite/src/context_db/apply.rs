use super::*;

mod apply_derivation;
mod apply_ingest;
mod apply_maintenance;
mod apply_projection;
mod apply_replay;

impl ContextDb {
    pub fn apply_ingest(&self, plan: &IngestPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_ingest_in_tx(tx, plan))
    }

    pub fn delete_raw_events(&self, stable_ids: &[String]) -> Result<usize> {
        self.with_write_tx(|tx| Self::delete_raw_events_in_tx(tx, stable_ids))
    }

    pub fn delete_source_cursors_for_connector(&self, connector: &str) -> Result<usize> {
        self.with_write_tx(|tx| Self::delete_source_cursors_for_connector_in_tx(tx, connector))
    }

    pub fn delete_import_journal_for_connector(&self, connector: &str) -> Result<usize> {
        self.with_write_tx(|tx| Self::delete_import_journal_for_connector_in_tx(tx, connector))
    }

    pub fn clear_derived_state(&self) -> Result<()> {
        self.with_write_tx(Self::clear_derived_state_in_tx)
    }

    pub fn apply_projection(&self, plan: &ProjectionPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_projection_in_tx(tx, plan))
    }

    pub fn apply_derivation(&self, plan: &DerivePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_derivation_in_tx(tx, plan))
    }
}
