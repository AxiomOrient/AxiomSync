use super::*;

impl ContextDb {
    pub fn existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, native_session_id, native_event_id, event_type, ts_ms, hex(payload_sha256)
             from raw_event",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                let stable_id: String = row.get(0)?;
                let connector: String = row.get(1)?;
                let native_session_id: String = row.get(2)?;
                let native_event_id: Option<String> = row.get(3)?;
                let event_type: String = row.get(4)?;
                let ts_ms: i64 = row.get(5)?;
                let payload_sha256_hex: String = row.get(6)?;
                let dedupe_key = make_stable_id(
                    "dedupe",
                    &serde_json::json!({
                        "connector": connector,
                        "native_session_id": native_session_id,
                        "native_event_id": native_event_id,
                        "event_type": event_type,
                        "ts_ms": ts_ms,
                        "payload_sha256": payload_sha256_hex.to_ascii_lowercase(),
                    }),
                );
                Ok(ExistingRawEventKey {
                    stable_id,
                    dedupe_key,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_raw_events(&self) -> Result<Vec<RawEventRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, native_schema_version, native_session_id, native_event_id, event_type, ts_ms, payload_json, hex(payload_sha256)
             from raw_event
             order by ts_ms asc, connector asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(RawEventRow {
                    stable_id: row.get(0)?,
                    connector: row.get(1)?,
                    native_schema_version: row.get(2)?,
                    native_session_id: row.get(3)?,
                    native_event_id: row.get(4)?,
                    event_type: row.get(5)?,
                    ts_ms: row.get(6)?,
                    payload_json: row.get(7)?,
                    payload_sha256_hex: row.get::<_, String>(8)?.to_ascii_lowercase(),
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "select connector, cursor_key, cursor_value, updated_at_ms
             from source_cursor order by connector, cursor_key",
            )
            .map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SourceCursorRow {
                    connector: row.get(0)?,
                    cursor_key: row.get(1)?,
                    cursor_value: row.get(2)?,
                    updated_at_ms: row.get(3)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }

    pub fn load_import_journal(&self) -> Result<Vec<ImportJournalRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, imported_events, skipped_events, cursor_key, cursor_value, applied_at_ms
             from import_journal
             order by applied_at_ms asc, stable_id asc",
        ).map_db_err()?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ImportJournalRow {
                    stable_id: row.get(0)?,
                    connector: row.get(1)?,
                    imported_events: row.get::<_, i64>(2)? as usize,
                    skipped_events: row.get::<_, i64>(3)? as usize,
                    cursor_key: row.get(4)?,
                    cursor_value: row.get(5)?,
                    applied_at_ms: row.get(6)?,
                })
            })
            .map_db_err()?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_error)
    }
}
