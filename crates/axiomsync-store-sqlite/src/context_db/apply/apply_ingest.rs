use super::*;

impl ContextDb {
    pub(crate) fn apply_ingest_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &IngestPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        for event in &plan.adds {
            tx.execute(
                "insert or ignore into raw_event
                 (stable_id, connector, native_schema_version, native_session_id, native_event_id, event_type, ts_ms, payload_json, payload_sha256)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    event.row.stable_id,
                    event.row.connector,
                    event.row.native_schema_version,
                    event.row.native_session_id,
                    event.row.native_event_id,
                    event.row.event_type,
                    event.row.ts_ms,
                    event.row.payload_json,
                    hex_to_bytes(&event.row.payload_sha256_hex)?,
                ],
            )
            .map_db_err()?;
        }
        if let Some(journal) = &plan.journal {
            tx.execute(
                "insert into import_journal
                 (stable_id, connector, imported_events, skipped_events, cursor_key, cursor_value, applied_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    journal.stable_id,
                    journal.connector,
                    journal.imported_events as i64,
                    journal.skipped_events as i64,
                    journal.cursor_key,
                    journal.cursor_value,
                    journal.applied_at_ms
                ],
            )
            .map_db_err()?;
        }
        if let Some(cursor) = &plan.cursor_update {
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
        }
        Ok(serde_json::to_value(plan)?)
    }
}
