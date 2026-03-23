use super::*;

impl ContextDb {
    pub(crate) fn upsert_source_cursor_in_tx(
        tx: &rusqlite::Transaction<'_>,
        cursor: &SourceCursorRow,
    ) -> Result<()> {
        cursor.validate()?;
        tx.execute(
            "insert into source_cursor (connector, cursor_key, cursor_value, updated_at_ms)
             values (?1, ?2, ?3, ?4)
             on conflict(connector, cursor_key) do update set
             cursor_value = excluded.cursor_value,
             updated_at_ms = excluded.updated_at_ms",
            params![
                cursor.connector,
                cursor.cursor_key,
                cursor.cursor_value,
                cursor.updated_at_ms
            ],
        )
        .map_db_err()?;
        Ok(())
    }

    pub(crate) fn apply_source_cursor_upsert_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &SourceCursorUpsertPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        Self::upsert_source_cursor_in_tx(tx, &plan.cursor)?;
        Ok(serde_json::to_value(plan)?)
    }
}
