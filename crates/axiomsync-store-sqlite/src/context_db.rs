use std::fs;
use std::path::{Path, PathBuf};

use axiomsync_domain::domain::{
    AnchorRow, ArtifactRow, ClaimEvidenceRow, ClaimRow, DerivePlan, DoctorReport, EntryRow,
    EpisodeRow, IngestPlan, IngressReceiptRow, ProcedureEvidenceRow, ProcedureRow, ProjectionPlan,
    SessionRow, SourceCursorRow, SourceCursorUpsertPlan, stable_id,
};
use axiomsync_domain::error::{AxiomError, Result};
use axiomsync_kernel::ports::RepositoryPort;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ContextDb {
    root: PathBuf,
    db_path: PathBuf,
}

#[derive(Debug, Clone)]
struct LegacyRawEventRow {
    stable_id: String,
    connector: String,
    native_schema_version: Option<String>,
    native_session_id: String,
    native_event_id: Option<String>,
    event_type: String,
    ts_ms: i64,
    payload_json: String,
    payload_sha256: Vec<u8>,
}

impl ContextDb {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let db = Self {
            db_path: root.join("context.db"),
            root,
        };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<()> {
        let mut conn = self.connection()?;
        conn.execute_batch(include_str!("schema.sql"))
            .map_err(map_db_err)?;
        self.migrate_legacy(&mut conn)?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path).map_err(map_db_err)
    }

    fn migrate_legacy(&self, conn: &mut Connection) -> Result<()> {
        if !table_exists(conn, "raw_event")? {
            return Ok(());
        }
        let current_count: i64 = conn
            .query_row("select count(*) from ingress_receipts", [], |row| row.get(0))
            .map_err(map_db_err)?;
        if current_count == 0 {
            let tx = conn.transaction().map_err(map_db_err)?;
            let rows = load_legacy_raw_events(&tx)?;
            let receipts = plan_legacy_backfill(&rows)?;
            apply_legacy_backfill(&tx, &receipts)?;
            tx.commit().map_err(map_db_err)?;
        }
        drop_legacy_tables(conn)?;
        Ok(())
    }

    fn with_tx<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut conn = self.connection()?;
        let tx = conn.transaction().map_err(map_db_err)?;
        let value = f(&tx)?;
        tx.commit().map_err(map_db_err)?;
        Ok(value)
    }
}

impl RepositoryPort for ContextDb {
    fn root(&self) -> &Path {
        &self.root
    }

    fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn init_report(&self) -> Result<Value> {
        Ok(json!({
            "root": self.root,
            "db_path": self.db_path,
            "schema_version": axiomsync_domain::domain::KERNEL_SCHEMA_VERSION,
        }))
    }

    fn existing_dedupe_keys(&self) -> Result<Vec<String>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare("select dedupe_key from ingress_receipts where dedupe_key is not null")
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_receipts(&self) -> Result<Vec<IngressReceiptRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select receipt_id, batch_id, source_kind, connector, session_kind,
                        external_session_key, external_entry_key, event_kind, observed_at,
                        captured_at, workspace_root, content_hash, dedupe_key, payload_json,
                        raw_payload_json, artifacts_json
                 from ingress_receipts
                 order by observed_at asc, receipt_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(IngressReceiptRow {
                    receipt_id: row.get(0)?,
                    batch_id: row.get(1)?,
                    source_kind: row.get(2)?,
                    connector: row.get(3)?,
                    session_kind: row.get(4)?,
                    external_session_key: row.get(5)?,
                    external_entry_key: row.get(6)?,
                    event_kind: row.get(7)?,
                    observed_at: row.get(8)?,
                    captured_at: row.get(9)?,
                    workspace_root: row.get(10)?,
                    content_hash: row.get(11)?,
                    dedupe_key: row.get(12)?,
                    payload_json: row.get(13)?,
                    raw_payload_json: row.get(14)?,
                    artifacts_json: row.get(15)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select connector, cursor_key, cursor_value, updated_at
                 from source_cursor
                 order by connector asc, cursor_key asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SourceCursorRow {
                    connector: row.get(0)?,
                    cursor_key: row.get(1)?,
                    cursor_value: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn apply_ingest(&self, plan: &IngestPlan) -> Result<Value> {
        self.with_tx(|tx| {
            plan.validate()?;
            for receipt in &plan.receipts {
                tx.execute(
                    "insert or ignore into ingress_receipts (
                        receipt_id, batch_id, source_kind, connector, session_kind,
                        external_session_key, external_entry_key, event_kind, observed_at,
                        captured_at, workspace_root, content_hash, dedupe_key, payload_json,
                        raw_payload_json, artifacts_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        receipt.receipt_id,
                        receipt.batch_id,
                        receipt.source_kind,
                        receipt.connector,
                        receipt.session_kind,
                        receipt.external_session_key,
                        receipt.external_entry_key,
                        receipt.event_kind,
                        receipt.observed_at,
                        receipt.captured_at,
                        receipt.workspace_root,
                        receipt.content_hash,
                        receipt.dedupe_key,
                        receipt.payload_json,
                        receipt.raw_payload_json,
                        receipt.artifacts_json,
                    ],
                )
                .map_err(map_db_err)?;
            }
            if let Some(cursor) = &plan.cursor_update {
                upsert_source_cursor_tx(tx, cursor)?;
            }
            Ok(json!({
                "accepted": plan.receipts.len(),
                "skipped": plan.skipped_dedupe_keys.len(),
            }))
        })
    }

    fn apply_source_cursor_upsert(&self, plan: &SourceCursorUpsertPlan) -> Result<Value> {
        self.with_tx(|tx| {
            upsert_source_cursor_tx(tx, &plan.cursor)?;
            Ok(json!({ "updated": true }))
        })
    }

    fn replace_projection(&self, plan: &ProjectionPlan) -> Result<Value> {
        self.with_tx(|tx| {
            tx.execute("delete from anchors", []).map_err(map_db_err)?;
            tx.execute("delete from artifacts", []).map_err(map_db_err)?;
            tx.execute("delete from entries", []).map_err(map_db_err)?;
            tx.execute("delete from actors", []).map_err(map_db_err)?;
            tx.execute("delete from sessions", []).map_err(map_db_err)?;
            tx.execute("delete from entry_search_fts", []).map_err(map_db_err)?;

            for session in &plan.sessions {
                tx.execute(
                    "insert into sessions (
                        session_id, session_kind, connector, external_session_key, title,
                        workspace_root, opened_at, closed_at, metadata_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        session.session_id,
                        session.session_kind,
                        session.connector,
                        session.external_session_key,
                        session.title,
                        session.workspace_root,
                        session.opened_at,
                        session.closed_at,
                        serde_json::to_string(&session.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
            }
            for actor in &plan.actors {
                tx.execute(
                    "insert into actors (actor_id, actor_kind, stable_key, display_name, metadata_json)
                     values (?1, ?2, ?3, ?4, ?5)",
                    params![
                        actor.actor_id,
                        actor.actor_kind,
                        actor.stable_key,
                        actor.display_name,
                        serde_json::to_string(&actor.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
            }
            for entry in &plan.entries {
                tx.execute(
                    "insert into entries (
                        entry_id, session_id, seq_no, entry_kind, actor_id, parent_entry_id,
                        external_entry_key, text_body, started_at, ended_at, metadata_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        entry.entry_id,
                        entry.session_id,
                        entry.seq_no,
                        entry.entry_kind,
                        entry.actor_id,
                        entry.parent_entry_id,
                        entry.external_entry_key,
                        entry.text_body,
                        entry.started_at,
                        entry.ended_at,
                        serde_json::to_string(&entry.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into entry_search_fts (entry_id, session_id, entry_kind, text_body)
                     values (?1, ?2, ?3, ?4)",
                    params![
                        entry.entry_id,
                        entry.session_id,
                        entry.entry_kind,
                        entry.text_body.clone().unwrap_or_default(),
                    ],
                )
                .map_err(map_db_err)?;
            }
            for artifact in &plan.artifacts {
                tx.execute(
                    "insert into artifacts (
                        artifact_id, session_id, entry_id, artifact_kind, uri, mime_type,
                        sha256, size_bytes, metadata_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        artifact.artifact_id,
                        artifact.session_id,
                        artifact.entry_id,
                        artifact.artifact_kind,
                        artifact.uri,
                        artifact.mime_type,
                        artifact.sha256,
                        artifact.size_bytes,
                        serde_json::to_string(&artifact.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
            }
            for anchor in &plan.anchors {
                tx.execute(
                    "insert into anchors (
                        anchor_id, entry_id, artifact_id, anchor_kind, locator_json,
                        preview_text, fingerprint
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        anchor.anchor_id,
                        anchor.entry_id,
                        anchor.artifact_id,
                        anchor.anchor_kind,
                        anchor.locator_json,
                        anchor.preview_text,
                        anchor.fingerprint,
                    ],
                )
                .map_err(map_db_err)?;
            }
            Ok(json!({
                "sessions": plan.sessions.len(),
                "entries": plan.entries.len(),
                "artifacts": plan.artifacts.len(),
                "anchors": plan.anchors.len(),
            }))
        })
    }

    fn replace_derivation(&self, plan: &DerivePlan) -> Result<Value> {
        self.with_tx(|tx| {
            tx.execute("delete from claim_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from claims", []).map_err(map_db_err)?;
            tx.execute("delete from procedures", []).map_err(map_db_err)?;
            tx.execute("delete from episodes", []).map_err(map_db_err)?;
            tx.execute("delete from episode_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from claim_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_search_fts", []).map_err(map_db_err)?;

            for episode in &plan.episodes {
                tx.execute(
                    "insert into episodes (
                        episode_id, session_id, episode_kind, summary, status, confidence,
                        extractor_version, stale
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        episode.episode_id,
                        episode.session_id,
                        episode.episode_kind,
                        episode.summary,
                        episode.status,
                        episode.confidence,
                        episode.extractor_version,
                        if episode.stale { 1 } else { 0 },
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into episode_search_fts (episode_id, episode_kind, summary)
                     values (?1, ?2, ?3)",
                    params![episode.episode_id, episode.episode_kind, episode.summary],
                )
                .map_err(map_db_err)?;
            }
            for claim in &plan.claims {
                tx.execute(
                    "insert into claims (claim_id, episode_id, claim_kind, statement, confidence, metadata_json)
                     values (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        claim.claim_id,
                        claim.episode_id,
                        claim.claim_kind,
                        claim.statement,
                        claim.confidence,
                        serde_json::to_string(&claim.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into claim_search_fts (claim_id, claim_kind, statement)
                     values (?1, ?2, ?3)",
                    params![claim.claim_id, claim.claim_kind, claim.statement],
                )
                .map_err(map_db_err)?;
            }
            for row in &plan.claim_evidence {
                tx.execute(
                    "insert into claim_evidence (claim_id, anchor_id, support_kind) values (?1, ?2, ?3)",
                    params![row.claim_id, row.anchor_id, row.support_kind],
                )
                .map_err(map_db_err)?;
            }
            for procedure in &plan.procedures {
                tx.execute(
                    "insert into procedures (
                        procedure_id, title, goal, steps_json, confidence,
                        extractor_version, stale
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        procedure.procedure_id,
                        procedure.title,
                        procedure.goal,
                        serde_json::to_string(&procedure.steps_json)?,
                        procedure.confidence,
                        procedure.extractor_version,
                        if procedure.stale { 1 } else { 0 },
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into procedure_search_fts (procedure_id, title, goal, steps_text)
                     values (?1, ?2, ?3, ?4)",
                    params![
                        procedure.procedure_id,
                        procedure.title,
                        procedure.goal,
                        procedure.steps_json.to_string(),
                    ],
                )
                .map_err(map_db_err)?;
            }
            for row in &plan.procedure_evidence {
                tx.execute(
                    "insert into procedure_evidence (procedure_id, anchor_id, support_kind)
                     values (?1, ?2, ?3)",
                    params![row.procedure_id, row.anchor_id, row.support_kind],
                )
                .map_err(map_db_err)?;
            }
            Ok(json!({
                "episodes": plan.episodes.len(),
                "claims": plan.claims.len(),
                "procedures": plan.procedures.len(),
            }))
        })
    }

    fn load_sessions(&self) -> Result<Vec<SessionRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select session_id, session_kind, connector, external_session_key, title,
                        workspace_root, opened_at, closed_at, metadata_json
                 from sessions
                 order by opened_at asc, session_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SessionRow {
                    session_id: row.get(0)?,
                    session_kind: row.get(1)?,
                    connector: row.get(2)?,
                    external_session_key: row.get(3)?,
                    title: row.get(4)?,
                    workspace_root: row.get(5)?,
                    opened_at: row.get(6)?,
                    closed_at: row.get(7)?,
                    metadata_json: parse_json_value(row.get::<_, String>(8)?)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_entries(&self) -> Result<Vec<EntryRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select entry_id, session_id, seq_no, entry_kind, actor_id, parent_entry_id,
                        external_entry_key, text_body, started_at, ended_at, metadata_json
                 from entries
                 order by session_id asc, seq_no asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EntryRow {
                    entry_id: row.get(0)?,
                    session_id: row.get(1)?,
                    seq_no: row.get(2)?,
                    entry_kind: row.get(3)?,
                    actor_id: row.get(4)?,
                    parent_entry_id: row.get(5)?,
                    external_entry_key: row.get(6)?,
                    text_body: row.get(7)?,
                    started_at: row.get(8)?,
                    ended_at: row.get(9)?,
                    metadata_json: parse_json_value(row.get::<_, String>(10)?)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_artifacts(&self) -> Result<Vec<ArtifactRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select artifact_id, session_id, entry_id, artifact_kind, uri, mime_type,
                        sha256, size_bytes, metadata_json
                 from artifacts
                 order by artifact_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ArtifactRow {
                    artifact_id: row.get(0)?,
                    session_id: row.get(1)?,
                    entry_id: row.get(2)?,
                    artifact_kind: row.get(3)?,
                    uri: row.get(4)?,
                    mime_type: row.get(5)?,
                    sha256: row.get(6)?,
                    size_bytes: row.get(7)?,
                    metadata_json: parse_json_value(row.get::<_, String>(8)?)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_anchors(&self) -> Result<Vec<AnchorRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select anchor_id, entry_id, artifact_id, anchor_kind, locator_json,
                        preview_text, fingerprint
                 from anchors
                 order by anchor_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AnchorRow {
                    anchor_id: row.get(0)?,
                    entry_id: row.get(1)?,
                    artifact_id: row.get(2)?,
                    anchor_kind: row.get(3)?,
                    locator_json: row.get(4)?,
                    preview_text: row.get(5)?,
                    fingerprint: row.get(6)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_episodes(&self) -> Result<Vec<EpisodeRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select episode_id, session_id, episode_kind, summary, status, confidence,
                        extractor_version, stale
                 from episodes
                 order by episode_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EpisodeRow {
                    episode_id: row.get(0)?,
                    session_id: row.get(1)?,
                    episode_kind: row.get(2)?,
                    summary: row.get(3)?,
                    status: row.get(4)?,
                    confidence: row.get(5)?,
                    extractor_version: row.get(6)?,
                    stale: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_claims(&self) -> Result<Vec<ClaimRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select claim_id, episode_id, claim_kind, statement, confidence, metadata_json
                 from claims
                 order by claim_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ClaimRow {
                    claim_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    claim_kind: row.get(2)?,
                    statement: row.get(3)?,
                    confidence: row.get(4)?,
                    metadata_json: parse_json_value(row.get::<_, String>(5)?)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_claim_evidence(&self) -> Result<Vec<ClaimEvidenceRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select claim_id, anchor_id, support_kind
                 from claim_evidence
                 order by claim_id asc, anchor_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ClaimEvidenceRow {
                    claim_id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    support_kind: row.get(2)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_procedures(&self) -> Result<Vec<ProcedureRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select procedure_id, title, goal, steps_json, confidence, extractor_version, stale
                 from procedures
                 order by procedure_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProcedureRow {
                    procedure_id: row.get(0)?,
                    title: row.get(1)?,
                    goal: row.get(2)?,
                    steps_json: parse_json_value(row.get::<_, String>(3)?)?,
                    confidence: row.get(4)?,
                    extractor_version: row.get(5)?,
                    stale: row.get::<_, i64>(6)? != 0,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_procedure_evidence(&self) -> Result<Vec<ProcedureEvidenceRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select procedure_id, anchor_id, support_kind
                 from procedure_evidence
                 order by procedure_id asc, anchor_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProcedureEvidenceRow {
                    procedure_id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    support_kind: row.get(2)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn doctor_report(&self) -> Result<DoctorReport> {
        let conn = self.connection()?;
        Ok(DoctorReport {
            db_path: self.db_path.display().to_string(),
            schema_version: axiomsync_domain::domain::KERNEL_SCHEMA_VERSION.to_string(),
            ingress_receipts: count_rows(&conn, "ingress_receipts")?,
            sessions: count_rows(&conn, "sessions")?,
            entries: count_rows(&conn, "entries")?,
            episodes: count_rows(&conn, "episodes")?,
            claims: count_rows(&conn, "claims")?,
            procedures: count_rows(&conn, "procedures")?,
        })
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let found = conn
        .query_row(
            "select 1 from sqlite_master where type = 'table' and name = ?1",
            params![table],
            |_| Ok(()),
        )
        .optional()
        .map_err(map_db_err)?;
    Ok(found.is_some())
}

fn load_legacy_raw_events(tx: &Transaction<'_>) -> Result<Vec<LegacyRawEventRow>> {
    let mut stmt = tx
        .prepare(
            "select stable_id, connector, native_schema_version, native_session_id,
                    native_event_id, event_type, ts_ms, payload_json, payload_sha256
             from raw_event
             order by id asc",
        )
        .map_err(map_db_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyRawEventRow {
                stable_id: row.get(0)?,
                connector: row.get(1)?,
                native_schema_version: row.get(2)?,
                native_session_id: row.get(3)?,
                native_event_id: row.get(4)?,
                event_type: row.get(5)?,
                ts_ms: row.get(6)?,
                payload_json: row.get(7)?,
                payload_sha256: row.get(8)?,
            })
        })
        .map_err(map_db_err)?;
    rows.map(|row| row.map_err(map_db_err)).collect()
}

fn plan_legacy_backfill(rows: &[LegacyRawEventRow]) -> Result<Vec<IngressReceiptRow>> {
    rows.iter()
        .map(|row| {
            Ok(IngressReceiptRow {
                receipt_id: stable_id("receipt", &(row.stable_id.as_str(), row.connector.as_str())),
                batch_id: stable_id("batch", &"legacy"),
                source_kind: row.connector.clone(),
                connector: row.connector.clone(),
                session_kind: infer_legacy_session_kind(&row.event_type, &row.payload_json),
                external_session_key: Some(row.native_session_id.clone()),
                external_entry_key: row.native_event_id.clone(),
                event_kind: row.event_type.clone(),
                observed_at: axiomsync_domain::domain::ts_ms_to_rfc3339(row.ts_ms)?,
                captured_at: None,
                workspace_root: None,
                content_hash: hex::encode(&row.payload_sha256),
                dedupe_key: Some(row.stable_id.clone()),
                payload_json: row.payload_json.clone(),
                raw_payload_json: row.native_schema_version.clone(),
                artifacts_json: "[]".to_string(),
            })
        })
        .collect()
}

fn apply_legacy_backfill(tx: &Transaction<'_>, receipts: &[IngressReceiptRow]) -> Result<()> {
    for receipt in receipts {
        tx.execute(
            "insert or ignore into ingress_receipts (
                receipt_id, batch_id, source_kind, connector, session_kind,
                external_session_key, external_entry_key, event_kind, observed_at,
                captured_at, workspace_root, content_hash, dedupe_key, payload_json,
                raw_payload_json, artifacts_json
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                receipt.receipt_id,
                receipt.batch_id,
                receipt.source_kind,
                receipt.connector,
                receipt.session_kind,
                receipt.external_session_key,
                receipt.external_entry_key,
                receipt.event_kind,
                receipt.observed_at,
                receipt.captured_at,
                receipt.workspace_root,
                receipt.content_hash,
                receipt.dedupe_key,
                receipt.payload_json,
                receipt.raw_payload_json,
                receipt.artifacts_json,
            ],
        )
        .map_err(map_db_err)?;
    }
    Ok(())
}

fn drop_legacy_tables(conn: &Connection) -> Result<()> {
    for table in [
        "workspace",
        "raw_event",
        "import_journal",
        "conv_session",
        "conv_turn",
        "conv_item",
        "artifact",
        "evidence_anchor",
        "execution_run",
        "execution_task",
        "execution_check",
        "execution_approval",
        "execution_event",
        "document_record",
        "episode",
        "episode_member",
        "insight",
        "verification",
        "search_doc_redacted",
        "insight_fts",
        "search_doc_redacted_fts",
    ] {
        conn.execute(&format!("drop table if exists {table}"), [])
            .map_err(map_db_err)?;
    }
    Ok(())
}

fn count_rows(conn: &Connection, table: &str) -> Result<usize> {
    conn.query_row(&format!("select count(*) from {table}"), [], |row| row.get::<_, i64>(0))
        .map(|value| value as usize)
        .map_err(map_db_err)
}

fn upsert_source_cursor_tx(tx: &Transaction<'_>, cursor: &SourceCursorRow) -> Result<()> {
    tx.execute(
        "insert into source_cursor (connector, cursor_key, cursor_value, updated_at)
         values (?1, ?2, ?3, ?4)
         on conflict(connector, cursor_key) do update set
           cursor_value = excluded.cursor_value,
           updated_at = excluded.updated_at",
        params![
            cursor.connector,
            cursor.cursor_key,
            cursor.cursor_value,
            cursor.updated_at,
        ],
    )
    .map_err(map_db_err)?;
    Ok(())
}

fn parse_json_value(raw: String) -> std::result::Result<Value, rusqlite::Error> {
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            raw.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn map_db_err(error: rusqlite::Error) -> AxiomError {
    AxiomError::Internal(error.to_string())
}

fn infer_legacy_session_kind(event_type: &str, payload_json: &str) -> String {
    let lowered = event_type.to_ascii_lowercase();
    if lowered.contains("run") || lowered.contains("task") || lowered.contains("approval") {
        "run".to_string()
    } else if lowered.contains("document") {
        "import".to_string()
    } else if payload_json.contains("\"subject\":{\"kind\":\"document\"") {
        "import".to_string()
    } else {
        "conversation".to_string()
    }
}
