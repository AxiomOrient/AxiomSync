use super::*;

impl ContextDb {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let db_path = root.join("context.db");
        let store = Self { root, db_path };
        store.initialize()?;
        Ok(store)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub(crate) fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path).map_db_err()?;
        conn.execute_batch("pragma foreign_keys = on; pragma journal_mode = wal;")
            .map_db_err()?;
        Ok(conn)
    }

    pub(crate) fn with_write_tx<T, F>(&self, apply: F) -> Result<T>
    where
        F: for<'tx> FnOnce(&rusqlite::Transaction<'tx>) -> Result<T>,
    {
        let mut conn = self.connect()?;
        let tx = conn.transaction().map_db_err()?;
        let output = apply(&tx)?;
        tx.commit().map_db_err()?;
        Ok(output)
    }

    fn initialize(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(BASE_SCHEMA_SQL).map_db_err()?;
        add_stable_id_column(&conn, "workspace")?;
        add_stable_id_column(&conn, "raw_event")?;
        add_stable_id_column(&conn, "conv_session")?;
        add_stable_id_column(&conn, "conv_turn")?;
        add_stable_id_column(&conn, "conv_item")?;
        add_stable_id_column(&conn, "artifact")?;
        add_stable_id_column(&conn, "evidence_anchor")?;
        add_stable_id_column(&conn, "episode")?;
        add_stable_id_column(&conn, "insight")?;
        add_stable_id_column(&conn, "verification")?;
        add_stable_id_column(&conn, "import_journal")?;
        add_stable_id_column(&conn, "search_doc_redacted")?;
        conn.execute_batch(EXTRA_SCHEMA_SQL).map_db_err()?;
        conn.execute(
            "insert into axiomsync_meta(key, value) values ('schema_version', ?1)
             on conflict(key) do update set value = excluded.value",
            [crate::domain::RENEWAL_SCHEMA_VERSION],
        )
        .map_db_err()?;
        Ok(())
    }

    pub fn init_report(&self) -> Result<serde_json::Value> {
        let conn = self.connect()?;
        let count: i64 = conn
            .query_row(
                "select count(*) from sqlite_master where type = 'table'",
                [],
                |row| row.get(0),
            )
            .map_db_err()?;
        let doctor = self.doctor_report()?;
        Ok(serde_json::json!({
            "status": "ok",
            "root": self.root,
            "db_path": self.db_path,
            "tables": count,
            "schema_version": crate::domain::RENEWAL_SCHEMA_VERSION,
            "drift_detected": doctor.drift_detected,
        }))
    }
}
