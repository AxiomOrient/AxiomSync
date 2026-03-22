use super::*;

impl ContextDb {
    pub(crate) fn apply_replay_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &ReplayPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        Self::clear_derived_state_in_tx(tx)?;
        let projection = Self::apply_projection_in_tx(tx, &plan.projection)?;
        let derivation = Self::apply_derivation_in_tx(tx, &plan.derivation)?;
        Ok(serde_json::json!({
            "projection": projection,
            "derivation": derivation,
        }))
    }

    pub(crate) fn apply_purge_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &PurgePlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        Self::delete_raw_events_in_tx(tx, &plan.deleted_raw_event_ids)?;
        if let Some(connector) = plan.connector.as_deref() {
            Self::delete_source_cursors_for_connector_in_tx(tx, connector)?;
            Self::delete_import_journal_for_connector_in_tx(tx, connector)?;
        }
        let replay = ReplayPlan {
            projection: plan.projection.clone(),
            derivation: plan.derivation.clone(),
        };
        let applied = Self::apply_replay_in_tx(tx, &replay)?;
        Ok(serde_json::json!({
            "deleted_raw_events": plan.deleted_raw_event_ids.len(),
            "applied": applied,
        }))
    }

    pub(crate) fn apply_repair_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &RepairPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        Self::apply_ingest_in_tx(tx, &plan.ingest)?;
        let applied = Self::apply_replay_in_tx(tx, &plan.replay)?;
        Ok(serde_json::json!({
            "ingest": plan.ingest.adds.len(),
            "applied": applied,
        }))
    }
}
