use super::*;

impl ContextDb {
    pub fn doctor_report(&self) -> Result<DoctorReport> {
        let conn = self.connect()?;
        let tables = list_sqlite_objects(&conn, "table")?;
        let indexes = list_sqlite_objects(&conn, "index")?;
        let required_tables = vec![
            "axiomsync_meta",
            "workspace",
            "raw_event",
            "source_cursor",
            "import_journal",
            "conv_session",
            "conv_turn",
            "conv_item",
            "artifact",
            "evidence_anchor",
            "episode",
            "episode_member",
            "insight",
            "verification",
            "search_doc_redacted",
            "insight_anchor",
        ];
        let required_indexes = vec![
            "idx_workspace_stable_id",
            "idx_raw_event_stable_id",
            "idx_conv_session_stable_id",
            "idx_conv_turn_stable_id",
            "idx_conv_item_stable_id",
            "idx_artifact_stable_id",
            "idx_evidence_anchor_stable_id",
            "idx_episode_stable_id",
            "idx_insight_stable_id",
            "idx_verification_stable_id",
            "idx_import_journal_stable_id",
            "idx_search_doc_redacted_stable_id",
        ];
        let missing_tables = required_tables
            .into_iter()
            .filter(|name| !tables.iter().any(|table| table == name))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let missing_indexes = required_indexes
            .into_iter()
            .filter(|name| !indexes.iter().any(|index| index == name))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let stored_schema_version: Option<String> = conn
            .query_row(
                "select value from axiomsync_meta where key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_db_err()?;
        let version_mismatch =
            stored_schema_version.as_deref() != Some(crate::domain::RENEWAL_SCHEMA_VERSION);
        let insight_count: i64 = conn
            .query_row("select count(*) from insight", [], |row| row.get(0))
            .map_db_err()?;
        let insight_fts_count: i64 = conn
            .query_row("select count(*) from insight_fts", [], |row| row.get(0))
            .unwrap_or_default();
        let search_doc_count: i64 = conn
            .query_row("select count(*) from search_doc_redacted", [], |row| {
                row.get(0)
            })
            .map_db_err()?;
        let search_doc_fts_count: i64 = conn
            .query_row("select count(*) from search_doc_redacted_fts", [], |row| {
                row.get(0)
            })
            .unwrap_or_default();
        let fts_rebuild_required =
            insight_count != insight_fts_count || search_doc_count != search_doc_fts_count;
        Ok(DoctorReport {
            schema_version: crate::domain::RENEWAL_SCHEMA_VERSION.to_string(),
            stored_schema_version,
            version_mismatch,
            fts_rebuild_required,
            drift_detected: !missing_tables.is_empty()
                || !missing_indexes.is_empty()
                || version_mismatch
                || fts_rebuild_required,
            missing_tables,
            missing_indexes,
        })
    }
}
