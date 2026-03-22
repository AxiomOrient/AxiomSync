use super::*;

impl ContextDb {
    pub(crate) fn delete_raw_events_in_tx(
        tx: &rusqlite::Transaction<'_>,
        stable_ids: &[String],
    ) -> Result<usize> {
        if stable_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = std::iter::repeat_n("?", stable_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("delete from raw_event where stable_id in ({placeholders})");
        tx.execute(&sql, rusqlite::params_from_iter(stable_ids.iter()))
            .map_db_err()
    }

    pub(crate) fn delete_source_cursors_for_connector_in_tx(
        tx: &rusqlite::Transaction<'_>,
        connector: &str,
    ) -> Result<usize> {
        tx.execute(
            "delete from source_cursor where connector = ?1",
            [connector],
        )
        .map_db_err()
    }

    pub(crate) fn delete_import_journal_for_connector_in_tx(
        tx: &rusqlite::Transaction<'_>,
        connector: &str,
    ) -> Result<usize> {
        tx.execute(
            "delete from import_journal where connector = ?1",
            [connector],
        )
        .map_db_err()
    }

    pub(crate) fn clear_derived_state_in_tx(tx: &rusqlite::Transaction<'_>) -> Result<()> {
        tx.execute("delete from search_doc_redacted", [])
            .map_db_err()?;
        tx.execute("delete from insight_anchor", []).map_db_err()?;
        tx.execute("delete from verification", []).map_db_err()?;
        tx.execute("delete from insight", []).map_db_err()?;
        tx.execute("delete from episode_member", []).map_db_err()?;
        tx.execute("delete from episode", []).map_db_err()?;
        Ok(())
    }
}
