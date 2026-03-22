use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::stable_id as make_stable_id;
use crate::domain::{
    ArtifactRow, ConvItemRow, ConvSessionRow, ConvTurnRow, DerivePlan, DoctorReport,
    EpisodeConnectorRow, EpisodeEvidenceSearchRow, EpisodeMemberRow, EpisodeRow, EvidenceAnchorRow,
    ExistingRawEventKey, ImportJournalRow, IngestPlan, InsightAnchorRow, InsightRow,
    ProjectionPlan, PurgePlan, RawEventRow, RepairPlan, ReplayPlan, SearchCommandCandidateRow,
    SearchDocRedactedRow, SearchEpisodeFtsRow, SourceCursorRow, ThreadItemView, ThreadTurnView,
    ThreadView, VerificationRow, WorkspaceRow,
};
use crate::error::{AxiomError, Result};
use crate::ports::{ReadRepository, TransactionManager, WriteRepository};

const BASE_SCHEMA_SQL: &str = include_str!("schema.sql");

const EXTRA_SCHEMA_SQL: &str = r#"
create table if not exists insight_anchor (
  insight_id integer not null references insight(id),
  anchor_id integer not null references evidence_anchor(id),
  primary key (insight_id, anchor_id)
);

create table if not exists axiomsync_meta (
  key text primary key,
  value text not null
);

create unique index if not exists idx_workspace_stable_id on workspace(stable_id);
create unique index if not exists idx_raw_event_stable_id on raw_event(stable_id);
create unique index if not exists idx_conv_session_stable_id on conv_session(stable_id);
create unique index if not exists idx_conv_turn_stable_id on conv_turn(stable_id);
create unique index if not exists idx_conv_item_stable_id on conv_item(stable_id);
create unique index if not exists idx_artifact_stable_id on artifact(stable_id);
create unique index if not exists idx_evidence_anchor_stable_id on evidence_anchor(stable_id);
create unique index if not exists idx_episode_stable_id on episode(stable_id);
create unique index if not exists idx_insight_stable_id on insight(stable_id);
create unique index if not exists idx_verification_stable_id on verification(stable_id);
create unique index if not exists idx_import_journal_stable_id on import_journal(stable_id);
create unique index if not exists idx_search_doc_redacted_stable_id on search_doc_redacted(stable_id);
"#;

#[derive(Debug, Clone)]
pub struct ContextDb {
    root: PathBuf,
    db_path: PathBuf,
}

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

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("pragma foreign_keys = on; pragma journal_mode = wal;")?;
        Ok(conn)
    }

    fn with_write_tx<T, F>(&self, apply: F) -> Result<T>
    where
        F: for<'tx> FnOnce(&rusqlite::Transaction<'tx>) -> Result<T>,
    {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        let output = apply(&tx)?;
        tx.commit()?;
        Ok(output)
    }

    fn apply_ingest_in_tx(
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
            )?;
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
            )?;
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
            )?;
        }
        Ok(serde_json::to_value(plan)?)
    }

    fn delete_raw_events_in_tx(
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
            .map_err(Into::into)
    }

    fn delete_source_cursors_for_connector_in_tx(
        tx: &rusqlite::Transaction<'_>,
        connector: &str,
    ) -> Result<usize> {
        tx.execute(
            "delete from source_cursor where connector = ?1",
            [connector],
        )
        .map_err(Into::into)
    }

    fn delete_import_journal_for_connector_in_tx(
        tx: &rusqlite::Transaction<'_>,
        connector: &str,
    ) -> Result<usize> {
        tx.execute(
            "delete from import_journal where connector = ?1",
            [connector],
        )
        .map_err(Into::into)
    }

    fn clear_derived_state_in_tx(tx: &rusqlite::Transaction<'_>) -> Result<()> {
        tx.execute("delete from search_doc_redacted", [])?;
        tx.execute("delete from insight_anchor", [])?;
        tx.execute("delete from verification", [])?;
        tx.execute("delete from insight", [])?;
        tx.execute("delete from episode_member", [])?;
        tx.execute("delete from episode", [])?;
        Ok(())
    }

    fn apply_projection_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &ProjectionPlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        tx.execute("delete from artifact", [])?;
        tx.execute("delete from evidence_anchor", [])?;
        tx.execute("delete from conv_item", [])?;
        tx.execute("delete from conv_turn", [])?;
        tx.execute("delete from conv_session", [])?;
        tx.execute("delete from workspace", [])?;

        for workspace in &plan.workspaces {
            tx.execute(
                "insert into workspace (stable_id, canonical_root, repo_remote, branch, worktree_path)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    workspace.stable_id,
                    workspace.canonical_root,
                    workspace.repo_remote,
                    workspace.branch,
                    workspace.worktree_path
                ],
            )?;
        }
        let workspace_ids = stable_id_map(tx, "workspace")?;

        for session in &plan.conv_sessions {
            tx.execute(
                "insert into conv_session (stable_id, connector, native_session_id, workspace_id, title, transcript_uri, status, started_at_ms, ended_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    session.stable_id,
                    session.connector,
                    session.native_session_id,
                    lookup_fk(&workspace_ids, session.workspace_id.as_deref())?,
                    session.title,
                    session.transcript_uri,
                    session.status,
                    session.started_at_ms,
                    session.ended_at_ms
                ],
            )?;
        }
        let session_ids = stable_id_map(tx, "conv_session")?;

        for turn in &plan.conv_turns {
            tx.execute(
                "insert into conv_turn (stable_id, session_id, native_turn_id, turn_index, actor)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    turn.stable_id,
                    lookup_fk(&session_ids, Some(turn.session_id.as_str()))?,
                    turn.native_turn_id,
                    turn.turn_index as i64,
                    turn.actor
                ],
            )?;
        }
        let turn_ids = stable_id_map(tx, "conv_turn")?;

        for item in &plan.conv_items {
            tx.execute(
                "insert into conv_item (stable_id, turn_id, item_type, tool_name, body_text, payload_json)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item.stable_id,
                    lookup_fk(&turn_ids, Some(item.turn_id.as_str()))?,
                    item.item_type,
                    item.tool_name,
                    item.body_text,
                    item.payload_json
                ],
            )?;
        }
        let item_ids = stable_id_map(tx, "conv_item")?;

        for artifact in &plan.artifacts {
            tx.execute(
                "insert into artifact (stable_id, item_id, uri, mime, sha256, bytes)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    artifact.stable_id,
                    lookup_fk(&item_ids, Some(artifact.item_id.as_str()))?,
                    artifact.uri,
                    artifact.mime,
                    artifact
                        .sha256_hex
                        .as_deref()
                        .map(hex_to_bytes)
                        .transpose()?,
                    artifact.bytes.map(|value| value as i64)
                ],
            )?;
        }

        for anchor in &plan.evidence_anchors {
            tx.execute(
                "insert into evidence_anchor (stable_id, item_id, selector_type, selector_json, quoted_text)
                 values (?1, ?2, ?3, ?4, ?5)",
                params![
                    anchor.stable_id,
                    lookup_fk(&item_ids, Some(anchor.item_id.as_str()))?,
                    anchor.selector_type,
                    anchor.selector_json,
                    anchor.quoted_text
                ],
            )?;
        }

        Ok(serde_json::json!({
            "workspaces": plan.workspaces.len(),
            "conv_sessions": plan.conv_sessions.len(),
            "conv_turns": plan.conv_turns.len(),
            "conv_items": plan.conv_items.len(),
            "artifacts": plan.artifacts.len(),
            "evidence_anchors": plan.evidence_anchors.len(),
        }))
    }

    fn apply_derivation_in_tx(
        tx: &rusqlite::Transaction<'_>,
        plan: &DerivePlan,
    ) -> Result<serde_json::Value> {
        plan.validate()?;
        tx.execute("delete from search_doc_redacted", [])?;
        tx.execute("delete from insight_anchor", [])?;
        tx.execute("delete from verification", [])?;
        tx.execute("delete from insight", [])?;
        tx.execute("delete from episode_member", [])?;
        tx.execute("delete from episode", [])?;

        let workspace_ids = stable_id_map(tx, "workspace")?;
        let turn_ids = stable_id_map(tx, "conv_turn")?;
        let anchor_ids = stable_id_map(tx, "evidence_anchor")?;

        for episode in &plan.episodes {
            tx.execute(
                "insert into episode (stable_id, workspace_id, problem_signature, status, opened_at_ms, closed_at_ms)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    episode.stable_id,
                    lookup_fk(&workspace_ids, episode.workspace_id.as_deref())?,
                    episode.problem_signature,
                    episode.status,
                    episode.opened_at_ms,
                    episode.closed_at_ms
                ],
            )?;
        }
        let episode_ids = stable_id_map(tx, "episode")?;

        for member in &plan.episode_members {
            tx.execute(
                "insert into episode_member (episode_id, turn_id) values (?1, ?2)",
                params![
                    lookup_fk(&episode_ids, Some(member.episode_id.as_str()))?,
                    lookup_fk(&turn_ids, Some(member.turn_id.as_str()))?,
                ],
            )?;
        }

        for insight in &plan.insights {
            tx.execute(
                "insert into insight (stable_id, episode_id, kind, summary, normalized_text, extractor_version, confidence, stale)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    insight.stable_id,
                    lookup_fk(&episode_ids, Some(insight.episode_id.as_str()))?,
                    insight.kind,
                    insight.summary,
                    insight.normalized_text,
                    insight.extractor_version,
                    insight.confidence,
                    if insight.stale { 1 } else { 0 }
                ],
            )?;
        }
        let insight_ids = stable_id_map(tx, "insight")?;

        for link in &plan.insight_anchors {
            tx.execute(
                "insert into insight_anchor (insight_id, anchor_id) values (?1, ?2)",
                params![
                    lookup_fk(&insight_ids, Some(link.insight_id.as_str()))?,
                    lookup_fk(&anchor_ids, Some(link.anchor_id.as_str()))?,
                ],
            )?;
        }

        for verification in &plan.verifications {
            tx.execute(
                "insert into verification (stable_id, episode_id, kind, status, summary, evidence_id)
                 values (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    verification.stable_id,
                    lookup_fk(&episode_ids, Some(verification.episode_id.as_str()))?,
                    verification.kind,
                    verification.status,
                    verification.summary,
                    lookup_fk(&anchor_ids, verification.evidence_id.as_deref())?,
                ],
            )?;
        }
        for doc in &plan.search_docs_redacted {
            tx.execute(
                "insert into search_doc_redacted (stable_id, episode_id, body)
                 values (?1, ?2, ?3)",
                params![
                    doc.stable_id,
                    lookup_fk(&episode_ids, Some(doc.episode_id.as_str()))?,
                    doc.body,
                ],
            )?;
        }
        tx.execute(
            "insert into insight_fts(insight_fts) values ('rebuild')",
            [],
        )?;
        tx.execute(
            "insert into search_doc_redacted_fts(search_doc_redacted_fts) values ('rebuild')",
            [],
        )?;
        Ok(serde_json::json!({
            "episodes": plan.episodes.len(),
            "episode_members": plan.episode_members.len(),
            "insights": plan.insights.len(),
            "insight_anchors": plan.insight_anchors.len(),
            "verifications": plan.verifications.len(),
            "search_docs_redacted": plan.search_docs_redacted.len(),
        }))
    }

    fn apply_replay_in_tx(
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

    fn apply_purge_in_tx(
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

    fn apply_repair_in_tx(
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

    fn initialize(&self) -> Result<()> {
        let conn = self.connect()?;
        conn.execute_batch(BASE_SCHEMA_SQL)?;
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
        conn.execute_batch(EXTRA_SCHEMA_SQL)?;
        conn.execute(
            "insert into axiomsync_meta(key, value) values ('schema_version', ?1)
             on conflict(key) do update set value = excluded.value",
            [crate::domain::RENEWAL_SCHEMA_VERSION],
        )?;
        Ok(())
    }

    pub fn init_report(&self) -> Result<serde_json::Value> {
        let conn = self.connect()?;
        let count: i64 = conn.query_row(
            "select count(*) from sqlite_master where type = 'table'",
            [],
            |row| row.get(0),
        )?;
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

    pub fn existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, native_session_id, native_event_id, event_type, ts_ms, hex(payload_sha256)
             from raw_event",
        )?;
        let rows = stmt.query_map([], |row| {
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
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn apply_ingest(&self, plan: &IngestPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_ingest_in_tx(tx, plan))
    }

    pub fn load_raw_events(&self) -> Result<Vec<RawEventRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, native_schema_version, native_session_id, native_event_id, event_type, ts_ms, payload_json, hex(payload_sha256)
             from raw_event
             order by ts_ms asc, connector asc, stable_id asc",
        )?;
        let rows = stmt.query_map([], |row| {
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
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select connector, cursor_key, cursor_value, updated_at_ms
             from source_cursor order by connector, cursor_key",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SourceCursorRow {
                connector: row.get(0)?,
                cursor_key: row.get(1)?,
                cursor_value: row.get(2)?,
                updated_at_ms: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_import_journal(&self) -> Result<Vec<ImportJournalRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, connector, imported_events, skipped_events, cursor_key, cursor_value, applied_at_ms
             from import_journal
             order by applied_at_ms asc, stable_id asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ImportJournalRow {
                stable_id: row.get(0)?,
                connector: row.get(1)?,
                imported_events: row.get::<_, i64>(2)? as usize,
                skipped_events: row.get::<_, i64>(3)? as usize,
                cursor_key: row.get(4)?,
                cursor_value: row.get(5)?,
                applied_at_ms: row.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
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
            .optional()?;
        let version_mismatch =
            stored_schema_version.as_deref() != Some(crate::domain::RENEWAL_SCHEMA_VERSION);
        let insight_count: i64 =
            conn.query_row("select count(*) from insight", [], |row| row.get(0))?;
        let insight_fts_count: i64 = conn
            .query_row("select count(*) from insight_fts", [], |row| row.get(0))
            .unwrap_or_default();
        let search_doc_count: i64 =
            conn.query_row("select count(*) from search_doc_redacted", [], |row| {
                row.get(0)
            })?;
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

    pub fn clear_derived_state(&self) -> Result<()> {
        self.with_write_tx(Self::clear_derived_state_in_tx)
    }

    pub fn apply_projection(&self, plan: &ProjectionPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_projection_in_tx(tx, plan))
    }

    pub fn load_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select stable_id, canonical_root, repo_remote, branch, worktree_path
             from workspace order by stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceRow {
                stable_id: row.get(0)?,
                canonical_root: row.get(1)?,
                repo_remote: row.get(2)?,
                branch: row.get(3)?,
                worktree_path: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_sessions(&self) -> Result<Vec<ConvSessionRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_session.stable_id, conv_session.connector, conv_session.native_session_id,
                    workspace.stable_id, conv_session.title, conv_session.transcript_uri,
                    conv_session.status, conv_session.started_at_ms, conv_session.ended_at_ms
             from conv_session
             left join workspace on workspace.id = conv_session.workspace_id
             order by conv_session.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ConvSessionRow {
                stable_id: row.get(0)?,
                connector: row.get(1)?,
                native_session_id: row.get(2)?,
                workspace_id: row.get(3)?,
                title: row.get(4)?,
                transcript_uri: row.get(5)?,
                status: row.get(6)?,
                started_at_ms: row.get(7)?,
                ended_at_ms: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_turns(&self) -> Result<Vec<ConvTurnRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_turn.stable_id, conv_session.stable_id, conv_turn.native_turn_id, conv_turn.turn_index, conv_turn.actor
             from conv_turn
             join conv_session on conv_session.id = conv_turn.session_id
             order by conv_session.stable_id, conv_turn.turn_index, conv_turn.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ConvTurnRow {
                stable_id: row.get(0)?,
                session_id: row.get(1)?,
                native_turn_id: row.get(2)?,
                turn_index: row.get::<_, i64>(3)? as usize,
                actor: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_items(&self) -> Result<Vec<ConvItemRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select conv_item.stable_id, conv_turn.stable_id, conv_item.item_type, conv_item.tool_name, conv_item.body_text, conv_item.payload_json
             from conv_item
             join conv_turn on conv_turn.id = conv_item.turn_id
             order by conv_turn.stable_id, conv_item.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ConvItemRow {
                stable_id: row.get(0)?,
                turn_id: row.get(1)?,
                item_type: row.get(2)?,
                tool_name: row.get(3)?,
                body_text: row.get(4)?,
                payload_json: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_artifacts(&self) -> Result<Vec<ArtifactRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select artifact.stable_id, conv_item.stable_id, artifact.uri, artifact.mime, hex(artifact.sha256), artifact.bytes
             from artifact
             join conv_item on conv_item.id = artifact.item_id
             order by artifact.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            let sha256: Option<String> = row.get(4)?;
            Ok(ArtifactRow {
                stable_id: row.get(0)?,
                item_id: row.get(1)?,
                uri: row.get(2)?,
                mime: row.get(3)?,
                sha256_hex: sha256.map(|value| value.to_ascii_lowercase()),
                bytes: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_evidence_anchors(&self) -> Result<Vec<EvidenceAnchorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select evidence_anchor.stable_id, conv_item.stable_id, evidence_anchor.selector_type, evidence_anchor.selector_json, evidence_anchor.quoted_text
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             order by evidence_anchor.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EvidenceAnchorRow {
                stable_id: row.get(0)?,
                item_id: row.get(1)?,
                selector_type: row.get(2)?,
                selector_json: row.get(3)?,
                quoted_text: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn apply_derivation(&self, plan: &DerivePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_derivation_in_tx(tx, plan))
    }

    pub fn load_episodes(&self) -> Result<Vec<EpisodeRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id, workspace.stable_id, episode.problem_signature, episode.status, episode.opened_at_ms, episode.closed_at_ms
             from episode
             left join workspace on workspace.id = episode.workspace_id
             order by episode.opened_at_ms, episode.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EpisodeRow {
                stable_id: row.get(0)?,
                workspace_id: row.get(1)?,
                problem_signature: row.get(2)?,
                status: row.get(3)?,
                opened_at_ms: row.get(4)?,
                closed_at_ms: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_episode_members(&self) -> Result<Vec<EpisodeMemberRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id, conv_turn.stable_id
             from episode_member
             join episode on episode.id = episode_member.episode_id
             join conv_turn on conv_turn.id = episode_member.turn_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EpisodeMemberRow {
                episode_id: row.get(0)?,
                turn_id: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_insights(&self) -> Result<Vec<InsightRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select insight.stable_id, episode.stable_id, insight.kind, insight.summary, insight.normalized_text,
                    insight.extractor_version, insight.confidence, insight.stale
             from insight
             join episode on episode.id = insight.episode_id
             order by insight.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InsightRow {
                stable_id: row.get(0)?,
                episode_id: row.get(1)?,
                kind: row.get(2)?,
                summary: row.get(3)?,
                normalized_text: row.get(4)?,
                extractor_version: row.get(5)?,
                confidence: row.get(6)?,
                stale: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select insight.stable_id, evidence_anchor.stable_id
             from insight_anchor
             join insight on insight.id = insight_anchor.insight_id
             join evidence_anchor on evidence_anchor.id = insight_anchor.anchor_id
             order by insight.stable_id, evidence_anchor.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(InsightAnchorRow {
                insight_id: row.get(0)?,
                anchor_id: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_verifications(&self) -> Result<Vec<VerificationRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select verification.stable_id, episode.stable_id, verification.kind, verification.status, verification.summary, evidence_anchor.stable_id
             from verification
             join episode on episode.id = verification.episode_id
             left join evidence_anchor on evidence_anchor.id = verification.evidence_id
             order by verification.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(VerificationRow {
                stable_id: row.get(0)?,
                episode_id: row.get(1)?,
                kind: row.get(2)?,
                status: row.get(3)?,
                summary: row.get(4)?,
                evidence_id: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_search_docs_redacted(&self) -> Result<Vec<SearchDocRedactedRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select search_doc_redacted.stable_id, episode.stable_id, search_doc_redacted.body
             from search_doc_redacted
             join episode on episode.id = search_doc_redacted.episode_id
             order by search_doc_redacted.stable_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SearchDocRedactedRow {
                stable_id: row.get(0)?,
                episode_id: row.get(1)?,
                body: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        let session = self
            .load_sessions()?
            .into_iter()
            .find(|row| row.stable_id == session_id)
            .ok_or_else(|| AxiomError::NotFound(format!("thread {session_id}")))?;
        let turns = self
            .load_turns()?
            .into_iter()
            .filter(|turn| turn.session_id == session_id)
            .collect::<Vec<_>>();
        let items = self.load_items()?;
        let artifacts = self.load_artifacts()?;
        let mut turn_views = Vec::new();
        for turn in turns {
            let item_views = items
                .iter()
                .filter(|item| item.turn_id == turn.stable_id)
                .cloned()
                .map(|item| ThreadItemView {
                    artifacts: artifacts
                        .iter()
                        .filter(|artifact| artifact.item_id == item.stable_id)
                        .cloned()
                        .collect(),
                    item,
                })
                .collect();
            turn_views.push(ThreadTurnView {
                turn,
                items: item_views,
            });
        }
        Ok(ThreadView {
            session,
            turns: turn_views,
        })
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<crate::domain::EvidenceView> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select evidence_anchor.stable_id, conv_item.stable_id, evidence_anchor.selector_type, evidence_anchor.selector_json, evidence_anchor.quoted_text,
                    conv_item.turn_id, conv_item.item_type, conv_item.tool_name, conv_item.body_text, conv_item.payload_json
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             where evidence_anchor.stable_id = ?1",
        )?;
        stmt.query_row([evidence_id], |row| {
            Ok(crate::domain::EvidenceView {
                evidence: EvidenceAnchorRow {
                    stable_id: row.get(0)?,
                    item_id: row.get(1)?,
                    selector_type: row.get(2)?,
                    selector_json: row.get(3)?,
                    quoted_text: row.get(4)?,
                },
                item: ConvItemRow {
                    stable_id: row.get(1)?,
                    turn_id: row.get(5)?,
                    item_type: row.get(6)?,
                    tool_name: row.get(7)?,
                    body_text: row.get(8)?,
                    payload_json: row.get(9)?,
                },
            })
        })
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => {
                AxiomError::NotFound(format!("evidence {evidence_id}"))
            }
            other => other.into(),
        })
    }

    pub fn load_episode_connectors(&self) -> Result<Vec<EpisodeConnectorRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id, conv_session.connector, conv_turn.turn_index
             from episode
             join episode_member on episode_member.episode_id = episode.id
             join conv_turn on conv_turn.id = episode_member.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             order by episode.stable_id asc, conv_turn.turn_index asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EpisodeConnectorRow {
                episode_id: row.get(0)?,
                connector: row.get(1)?,
                turn_index: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from episode
             left join workspace on workspace.id = episode.workspace_id
             where episode.stable_id = ?1",
            [episode_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn thread_workspace_id(&self, thread_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from conv_session
             left join workspace on workspace.id = conv_session.workspace_id
             where conv_session.stable_id = ?1",
            [thread_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        let conn = self.connect()?;
        conn.query_row(
            "select workspace.stable_id
             from evidence_anchor
             join conv_item on conv_item.id = evidence_anchor.item_id
             join conv_turn on conv_turn.id = conv_item.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             left join workspace on workspace.id = conv_session.workspace_id
             where evidence_anchor.stable_id = ?1",
            [evidence_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn load_episode_search_fts_rows(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchEpisodeFtsRow>> {
        let Some(normalized_query) = crate::domain::normalize_fts_query(query) else {
            return Ok(Vec::new());
        };
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode_id, workspace_id, connector, status, matched_kind, matched_summary, pass_boost
             from (
               select episode.stable_id as episode_id,
                      workspace.stable_id as workspace_id,
                      conv_session.connector as connector,
                      episode.status as status,
                      insight.kind as matched_kind,
                      insight.summary as matched_summary,
                      max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
               from insight_fts
               join insight on insight.id = insight_fts.rowid
               join episode on episode.id = insight.episode_id
               left join workspace on workspace.id = episode.workspace_id
               left join verification on verification.episode_id = episode.id
               left join episode_member on episode_member.episode_id = episode.id
               left join conv_turn on conv_turn.id = episode_member.turn_id
               left join conv_session on conv_session.id = conv_turn.session_id
               where insight_fts match ?1
               group by episode.id, workspace.stable_id, conv_session.connector, episode.status, insight.id
               union all
               select episode.stable_id as episode_id,
                      workspace.stable_id as workspace_id,
                      conv_session.connector as connector,
                      episode.status as status,
                      null as matched_kind,
                      search_doc_redacted.body as matched_summary,
                      max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
               from search_doc_redacted_fts
               join search_doc_redacted on search_doc_redacted.id = search_doc_redacted_fts.rowid
               join episode on episode.id = search_doc_redacted.episode_id
               left join workspace on workspace.id = episode.workspace_id
               left join verification on verification.episode_id = episode.id
               left join episode_member on episode_member.episode_id = episode.id
               left join conv_turn on conv_turn.id = episode_member.turn_id
               left join conv_session on conv_session.id = conv_turn.session_id
               where search_doc_redacted_fts match ?1
               group by episode.id, workspace.stable_id, conv_session.connector, episode.status, search_doc_redacted.id
             )
             order by pass_boost desc, episode_id asc
             limit ?2",
        )?;
        let rows = stmt.query_map(params![normalized_query, limit as i64], |row| {
            Ok(SearchEpisodeFtsRow {
                episode_id: row.get(0)?,
                workspace_id: row.get(1)?,
                connector: row.get(2)?,
                status: row.get(3)?,
                matched_kind: row.get(4)?,
                matched_summary: row.get(5)?,
                pass_boost: row.get::<_, i64>(6)? != 0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_command_search_candidates(&self) -> Result<Vec<SearchCommandCandidateRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id, workspace.stable_id, insight.summary
             from insight
             join episode on episode.id = insight.episode_id
             left join workspace on workspace.id = episode.workspace_id
             where insight.kind = 'command'
             order by episode.stable_id asc, insight.summary asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SearchCommandCandidateRow {
                episode_id: row.get(0)?,
                workspace_id: row.get(1)?,
                command: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_episode_evidence_search_rows(&self) -> Result<Vec<EpisodeEvidenceSearchRow>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            "select episode.stable_id,
                    workspace.stable_id,
                    conv_session.connector,
                    episode.status,
                    evidence_anchor.stable_id,
                    evidence_anchor.quoted_text,
                    conv_item.body_text,
                    max(case when verification.status = 'pass' then 1 else 0 end) as pass_boost
             from episode
             join episode_member on episode_member.episode_id = episode.id
             join conv_turn on conv_turn.id = episode_member.turn_id
             join conv_session on conv_session.id = conv_turn.session_id
             join conv_item on conv_item.turn_id = conv_turn.id
             join evidence_anchor on evidence_anchor.item_id = conv_item.id
             left join workspace on workspace.id = episode.workspace_id
             left join verification on verification.episode_id = episode.id
             group by episode.id, workspace.stable_id, conv_session.connector, episode.status, evidence_anchor.id, conv_item.id
             order by episode.stable_id asc, evidence_anchor.stable_id asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EpisodeEvidenceSearchRow {
                episode_id: row.get(0)?,
                workspace_id: row.get(1)?,
                connector: row.get(2)?,
                status: row.get(3)?,
                evidence_id: row.get(4)?,
                quoted_text: row.get(5)?,
                body_text: row.get(6)?,
                pass_boost: row.get::<_, i64>(7)? != 0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

impl ReadRepository for ContextDb {
    fn root(&self) -> &Path {
        ContextDb::root(self)
    }

    fn db_path(&self) -> &Path {
        ContextDb::db_path(self)
    }

    fn init_report(&self) -> Result<serde_json::Value> {
        ContextDb::init_report(self)
    }

    fn existing_raw_event_keys(&self) -> Result<Vec<ExistingRawEventKey>> {
        ContextDb::existing_raw_event_keys(self)
    }

    fn load_raw_events(&self) -> Result<Vec<RawEventRow>> {
        ContextDb::load_raw_events(self)
    }

    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        ContextDb::load_source_cursors(self)
    }

    fn load_import_journal(&self) -> Result<Vec<ImportJournalRow>> {
        ContextDb::load_import_journal(self)
    }

    fn load_workspaces(&self) -> Result<Vec<WorkspaceRow>> {
        ContextDb::load_workspaces(self)
    }

    fn load_sessions(&self) -> Result<Vec<ConvSessionRow>> {
        ContextDb::load_sessions(self)
    }

    fn load_turns(&self) -> Result<Vec<ConvTurnRow>> {
        ContextDb::load_turns(self)
    }

    fn load_items(&self) -> Result<Vec<ConvItemRow>> {
        ContextDb::load_items(self)
    }

    fn load_evidence_anchors(&self) -> Result<Vec<EvidenceAnchorRow>> {
        ContextDb::load_evidence_anchors(self)
    }

    fn load_episodes(&self) -> Result<Vec<EpisodeRow>> {
        ContextDb::load_episodes(self)
    }

    fn load_insights(&self) -> Result<Vec<InsightRow>> {
        ContextDb::load_insights(self)
    }

    fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>> {
        ContextDb::load_insight_anchors(self)
    }

    fn load_verifications(&self) -> Result<Vec<VerificationRow>> {
        ContextDb::load_verifications(self)
    }

    fn load_search_docs_redacted(&self) -> Result<Vec<SearchDocRedactedRow>> {
        ContextDb::load_search_docs_redacted(self)
    }

    fn get_thread(&self, session_id: &str) -> Result<ThreadView> {
        ContextDb::get_thread(self, session_id)
    }

    fn get_evidence(&self, evidence_id: &str) -> Result<crate::domain::EvidenceView> {
        ContextDb::get_evidence(self, evidence_id)
    }

    fn load_episode_connectors(&self) -> Result<Vec<EpisodeConnectorRow>> {
        ContextDb::load_episode_connectors(self)
    }

    fn load_episode_search_fts_rows(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchEpisodeFtsRow>> {
        ContextDb::load_episode_search_fts_rows(self, query, limit)
    }

    fn load_episode_evidence_search_rows(&self) -> Result<Vec<EpisodeEvidenceSearchRow>> {
        ContextDb::load_episode_evidence_search_rows(self)
    }

    fn load_command_search_candidates(&self) -> Result<Vec<SearchCommandCandidateRow>> {
        ContextDb::load_command_search_candidates(self)
    }

    fn episode_workspace_id(&self, episode_id: &str) -> Result<Option<String>> {
        ContextDb::episode_workspace_id(self, episode_id)
    }

    fn thread_workspace_id(&self, thread_id: &str) -> Result<Option<String>> {
        ContextDb::thread_workspace_id(self, thread_id)
    }

    fn evidence_workspace_id(&self, evidence_id: &str) -> Result<Option<String>> {
        ContextDb::evidence_workspace_id(self, evidence_id)
    }

    fn doctor_report(&self) -> Result<DoctorReport> {
        ContextDb::doctor_report(self)
    }
}

impl WriteRepository for ContextDb {
    fn delete_raw_events(&self, stable_ids: &[String]) -> Result<usize> {
        ContextDb::delete_raw_events(self, stable_ids)
    }

    fn delete_source_cursors_for_connector(&self, connector: &str) -> Result<usize> {
        ContextDb::delete_source_cursors_for_connector(self, connector)
    }

    fn delete_import_journal_for_connector(&self, connector: &str) -> Result<usize> {
        ContextDb::delete_import_journal_for_connector(self, connector)
    }

    fn clear_derived_state(&self) -> Result<()> {
        ContextDb::clear_derived_state(self)
    }
}

impl TransactionManager for ContextDb {
    fn apply_ingest_tx(&self, plan: &IngestPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_ingest_in_tx(tx, plan))
    }

    fn apply_projection_tx(&self, plan: &ProjectionPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_projection_in_tx(tx, plan))
    }

    fn apply_derivation_tx(&self, plan: &DerivePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_derivation_in_tx(tx, plan))
    }

    fn apply_replay_tx(&self, plan: &ReplayPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_replay_in_tx(tx, plan))
    }

    fn apply_purge_tx(&self, plan: &PurgePlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_purge_in_tx(tx, plan))
    }

    fn apply_repair_tx(&self, plan: &RepairPlan) -> Result<serde_json::Value> {
        self.with_write_tx(|tx| Self::apply_repair_in_tx(tx, plan))
    }
}

fn add_stable_id_column(conn: &Connection, table: &str) -> Result<()> {
    let exists = conn
        .prepare(&format!("pragma table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|column| column == "stable_id");
    if !exists {
        conn.execute(
            &format!("alter table {table} add column stable_id text not null default ''"),
            [],
        )?;
    }
    Ok(())
}

fn stable_id_map(tx: &rusqlite::Transaction<'_>, table: &str) -> Result<HashMap<String, i64>> {
    let mut stmt = tx.prepare(&format!("select id, stable_id from {table}"))?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let pairs = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(pairs
        .into_iter()
        .map(|(id, stable_id)| (stable_id, id))
        .collect())
}

fn lookup_fk(map: &HashMap<String, i64>, stable_id: Option<&str>) -> Result<Option<i64>> {
    match stable_id {
        Some(value) => map
            .get(value)
            .copied()
            .map(Some)
            .ok_or_else(|| AxiomError::NotFound(format!("missing foreign key {value}"))),
        None => Ok(None),
    }
}

fn hex_to_bytes(value: &str) -> Result<Vec<u8>> {
    hex::decode(value)
        .map_err(|error| AxiomError::Validation(format!("invalid hex payload: {error}")))
}

fn list_sqlite_objects(conn: &Connection, object_type: &str) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("select name from sqlite_master where type = ?1 order by name asc")?;
    let rows = stmt.query_map([object_type], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
