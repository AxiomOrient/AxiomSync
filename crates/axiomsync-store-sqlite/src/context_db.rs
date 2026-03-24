use std::fs;
use std::path::{Path, PathBuf};

use axiomsync_domain::error::{AxiomError, Result};
use axiomsync_domain::{
    AnchorRow, ArtifactRow, ClaimRow, DerivePlan, DoctorReport, EntryRow, EpisodeRow, IngestPlan,
    IngressReceiptRow, InsightAnchorRow, InsightRow, ProcedureRow, ProjectionPlan, ReplayPlan,
    SessionRow, SourceCursorRow, SourceCursorUpsertPlan, VerificationRow,
};
use axiomsync_kernel::ports::RepositoryPort;
use rusqlite::{Connection, Transaction, params};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ContextDb {
    root: PathBuf,
    db_path: PathBuf,
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
        let conn = self.connection()?;
        conn.execute_batch(include_str!("schema.sql"))
            .map_err(map_db_err)?;
        self.migrate_current(&conn)?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path).map_err(map_db_err)
    }

    fn migrate_current(&self, conn: &Connection) -> Result<()> {
        ensure_column(
            conn,
            "ingress_receipts",
            "normalized_json",
            "TEXT NOT NULL DEFAULT '{}'",
        )?;
        ensure_column(
            conn,
            "ingress_receipts",
            "projection_state",
            "TEXT NOT NULL DEFAULT 'pending'",
        )?;
        ensure_column(
            conn,
            "ingress_receipts",
            "derived_state",
            "TEXT NOT NULL DEFAULT 'pending'",
        )?;
        ensure_column(
            conn,
            "ingress_receipts",
            "index_state",
            "TEXT NOT NULL DEFAULT 'pending'",
        )?;
        ensure_column(
            conn,
            "source_cursor",
            "metadata_json",
            "TEXT NOT NULL DEFAULT '{}'",
        )?;
        ensure_column(
            conn,
            "procedures",
            "status",
            "TEXT NOT NULL DEFAULT 'active'",
        )?;
        ensure_column(conn, "procedures", "episode_id", "TEXT")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS insights (
                insight_id TEXT PRIMARY KEY,
                episode_id TEXT REFERENCES episodes(episode_id) ON DELETE CASCADE,
                insight_kind TEXT NOT NULL,
                statement TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.0,
                scope_json TEXT NOT NULL DEFAULT '{}',
                metadata_json TEXT NOT NULL DEFAULT '{}'
            );
            CREATE TABLE IF NOT EXISTS insight_anchors (
                insight_id TEXT NOT NULL REFERENCES insights(insight_id) ON DELETE CASCADE,
                anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id) ON DELETE CASCADE,
                PRIMARY KEY (insight_id, anchor_id)
            );
            CREATE TABLE IF NOT EXISTS verifications (
                verification_id TEXT PRIMARY KEY,
                subject_kind TEXT NOT NULL,
                subject_id TEXT NOT NULL,
                method TEXT NOT NULL,
                status TEXT NOT NULL,
                checked_at TEXT NOT NULL,
                checker TEXT,
                details_json TEXT NOT NULL DEFAULT '{}'
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS insight_search_fts USING fts5(
                insight_id UNINDEXED,
                insight_kind UNINDEXED,
                statement
            );
            CREATE TABLE IF NOT EXISTS search_docs (
                doc_id TEXT PRIMARY KEY,
                doc_kind TEXT NOT NULL,
                subject_kind TEXT NOT NULL,
                subject_id TEXT NOT NULL,
                title TEXT,
                body TEXT NOT NULL,
                metadata_json TEXT NOT NULL DEFAULT '{}'
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS search_docs_fts USING fts5(
                doc_id UNINDEXED,
                doc_kind UNINDEXED,
                subject_kind UNINDEXED,
                subject_id UNINDEXED,
                title,
                body
            );",
        )
        .map_err(map_db_err)?;
        Ok(())
    }

    fn with_tx<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
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
            "schema_version": axiomsync_domain::KERNEL_SCHEMA_VERSION,
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
                        raw_payload_json, artifacts_json, normalized_json, projection_state,
                        derived_state, index_state
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
                    normalized_json: row.get(16)?,
                    projection_state: row.get(17)?,
                    derived_state: row.get(18)?,
                    index_state: row.get(19)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_source_cursors(&self) -> Result<Vec<SourceCursorRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select connector, cursor_key, cursor_value, updated_at, metadata_json
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
                    metadata_json: parse_json_value(row.get::<_, String>(4)?)?,
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
                        raw_payload_json, artifacts_json, normalized_json, projection_state,
                        derived_state, index_state
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                        receipt.normalized_json,
                        receipt.projection_state,
                        receipt.derived_state,
                        receipt.index_state,
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
                "accepted_receipts": plan.receipts.iter().map(|row| row.receipt_id.clone()).collect::<Vec<_>>(),
                "skipped_dedupe_keys": plan.skipped_dedupe_keys.clone(),
            }))
        })
    }

    fn apply_source_cursor_upsert(&self, plan: &SourceCursorUpsertPlan) -> Result<Value> {
        self.with_tx(|tx| {
            upsert_source_cursor_tx(tx, &plan.cursor)?;
            Ok(json!({ "updated": true }))
        })
    }

    fn apply_replay(&self, plan: &ReplayPlan) -> Result<Value> {
        self.with_tx(|tx| {
            tx.execute("delete from anchors", []).map_err(map_db_err)?;
            tx.execute("delete from artifacts", []).map_err(map_db_err)?;
            tx.execute("delete from entries", []).map_err(map_db_err)?;
            tx.execute("delete from actors", []).map_err(map_db_err)?;
            tx.execute("delete from sessions", []).map_err(map_db_err)?;
            tx.execute("delete from entry_search_fts", []).map_err(map_db_err)?;

            for session in &plan.projection.sessions {
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
            for actor in &plan.projection.actors {
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
            for entry in &plan.projection.entries {
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
            for artifact in &plan.projection.artifacts {
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
            for anchor in &plan.projection.anchors {
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
            tx.execute(
                "update ingress_receipts
                 set projection_state = 'projected',
                     derived_state = 'pending',
                     index_state = 'pending'",
                [],
            )
            .map_err(map_db_err)?;

            tx.execute("delete from insight_anchors", []).map_err(map_db_err)?;
            tx.execute("delete from verifications", []).map_err(map_db_err)?;
            tx.execute("delete from claim_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from insights", []).map_err(map_db_err)?;
            tx.execute("delete from claims", []).map_err(map_db_err)?;
            tx.execute("delete from procedures", []).map_err(map_db_err)?;
            tx.execute("delete from episodes", []).map_err(map_db_err)?;
            tx.execute("delete from episode_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from insight_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from claim_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from search_docs", []).map_err(map_db_err)?;
            tx.execute("delete from search_docs_fts", []).map_err(map_db_err)?;

            for episode in &plan.derivation.episodes {
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
            for insight in &plan.derivation.insights {
                tx.execute(
                    "insert into insights (
                        insight_id, episode_id, insight_kind, statement, confidence, scope_json, metadata_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        insight.insight_id,
                        insight.episode_id,
                        insight.insight_kind,
                        insight.statement,
                        insight.confidence,
                        serde_json::to_string(&insight.scope_json)?,
                        serde_json::to_string(&insight.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into insight_search_fts (insight_id, insight_kind, statement)
                     values (?1, ?2, ?3)",
                    params![insight.insight_id, insight.insight_kind, insight.statement],
                )
                .map_err(map_db_err)?;
            }
            for row in &plan.derivation.insight_anchors {
                tx.execute(
                    "insert into insight_anchors (insight_id, anchor_id) values (?1, ?2)",
                    params![row.insight_id, row.anchor_id],
                )
                .map_err(map_db_err)?;
            }
            for verification in &plan.derivation.verifications {
                tx.execute(
                    "insert into verifications (
                        verification_id, subject_kind, subject_id, method, status, checked_at, checker, details_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        verification.verification_id,
                        verification.subject_kind,
                        verification.subject_id,
                        verification.method,
                        verification.status,
                        verification.checked_at,
                        verification.checker,
                        serde_json::to_string(&verification.details_json)?,
                    ],
                )
                .map_err(map_db_err)?;
            }
            for claim in &plan.derivation.claims {
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
            for row in &plan.derivation.claim_evidence {
                tx.execute(
                    "insert into claim_evidence (claim_id, anchor_id, support_kind) values (?1, ?2, ?3)",
                    params![row.claim_id, row.anchor_id, row.support_kind],
                )
                .map_err(map_db_err)?;
            }
            for procedure in &plan.derivation.procedures {
                tx.execute(
                    "insert into procedures (
                        procedure_id, episode_id, title, goal, steps_json, status, confidence,
                        extractor_version, stale
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        procedure.procedure_id,
                        procedure.episode_id,
                        procedure.title,
                        procedure.goal,
                        serde_json::to_string(&procedure.steps_json)?,
                        procedure
                            .status
                            .clone()
                            .unwrap_or_else(|| "active".to_string()),
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
            for row in &plan.derivation.procedure_evidence {
                tx.execute(
                    "insert into procedure_evidence (procedure_id, anchor_id, support_kind)
                     values (?1, ?2, ?3)",
                    params![row.procedure_id, row.anchor_id, row.support_kind],
                )
                .map_err(map_db_err)?;
            }
            for doc in &plan.derivation.search_docs {
                tx.execute(
                    "insert into search_docs (doc_id, doc_kind, subject_kind, subject_id, title, body, metadata_json)
                     values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        doc.doc_id,
                        doc.doc_kind,
                        doc.subject_kind,
                        doc.subject_id,
                        doc.title,
                        doc.body,
                        serde_json::to_string(&doc.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into search_docs_fts (doc_id, doc_kind, subject_kind, subject_id, title, body)
                     values (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        doc.doc_id,
                        doc.doc_kind,
                        doc.subject_kind,
                        doc.subject_id,
                        doc.title,
                        doc.body,
                    ],
                )
                .map_err(map_db_err)?;
            }
            tx.execute(
                "update ingress_receipts
                 set derived_state = 'derived',
                     index_state = 'indexed'
                 where projection_state = 'projected'",
                [],
            )
            .map_err(map_db_err)?;
            Ok(json!({
                "projection": {
                    "sessions": plan.projection.sessions.len(),
                    "entries": plan.projection.entries.len(),
                    "artifacts": plan.projection.artifacts.len(),
                    "anchors": plan.projection.anchors.len(),
                },
                "derivation": {
                    "episodes": plan.derivation.episodes.len(),
                    "insights": plan.derivation.insights.len(),
                    "verifications": plan.derivation.verifications.len(),
                    "claims": plan.derivation.claims.len(),
                    "procedures": plan.derivation.procedures.len(),
                    "search_docs": plan.derivation.search_docs.len(),
                },
            }))
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
            tx.execute(
                "update ingress_receipts
                 set projection_state = 'projected',
                     derived_state = 'pending',
                     index_state = 'pending'",
                [],
            )
            .map_err(map_db_err)?;
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
            tx.execute("delete from insight_anchors", []).map_err(map_db_err)?;
            tx.execute("delete from verifications", []).map_err(map_db_err)?;
            tx.execute("delete from claim_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_evidence", []).map_err(map_db_err)?;
            tx.execute("delete from insights", []).map_err(map_db_err)?;
            tx.execute("delete from claims", []).map_err(map_db_err)?;
            tx.execute("delete from procedures", []).map_err(map_db_err)?;
            tx.execute("delete from episodes", []).map_err(map_db_err)?;
            tx.execute("delete from episode_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from insight_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from claim_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from procedure_search_fts", []).map_err(map_db_err)?;
            tx.execute("delete from search_docs", []).map_err(map_db_err)?;
            tx.execute("delete from search_docs_fts", []).map_err(map_db_err)?;

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
            for insight in &plan.insights {
                tx.execute(
                    "insert into insights (
                        insight_id, episode_id, insight_kind, statement, confidence, scope_json, metadata_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        insight.insight_id,
                        insight.episode_id,
                        insight.insight_kind,
                        insight.statement,
                        insight.confidence,
                        serde_json::to_string(&insight.scope_json)?,
                        serde_json::to_string(&insight.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into insight_search_fts (insight_id, insight_kind, statement)
                     values (?1, ?2, ?3)",
                    params![insight.insight_id, insight.insight_kind, insight.statement],
                )
                .map_err(map_db_err)?;
            }
            for row in &plan.insight_anchors {
                tx.execute(
                    "insert into insight_anchors (insight_id, anchor_id) values (?1, ?2)",
                    params![row.insight_id, row.anchor_id],
                )
                .map_err(map_db_err)?;
            }
            for verification in &plan.verifications {
                tx.execute(
                    "insert into verifications (
                        verification_id, subject_kind, subject_id, method, status, checked_at, checker, details_json
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        verification.verification_id,
                        verification.subject_kind,
                        verification.subject_id,
                        verification.method,
                        verification.status,
                        verification.checked_at,
                        verification.checker,
                        serde_json::to_string(&verification.details_json)?,
                    ],
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
                        procedure_id, episode_id, title, goal, steps_json, status, confidence,
                        extractor_version, stale
                    ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        procedure.procedure_id,
                        procedure.episode_id,
                        procedure.title,
                        procedure.goal,
                        serde_json::to_string(&procedure.steps_json)?,
                        procedure
                            .status
                            .clone()
                            .unwrap_or_else(|| "active".to_string()),
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
            for doc in &plan.search_docs {
                tx.execute(
                    "insert into search_docs (doc_id, doc_kind, subject_kind, subject_id, title, body, metadata_json)
                     values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        doc.doc_id,
                        doc.doc_kind,
                        doc.subject_kind,
                        doc.subject_id,
                        doc.title,
                        doc.body,
                        serde_json::to_string(&doc.metadata_json)?,
                    ],
                )
                .map_err(map_db_err)?;
                tx.execute(
                    "insert into search_docs_fts (doc_id, doc_kind, subject_kind, subject_id, title, body)
                     values (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        doc.doc_id,
                        doc.doc_kind,
                        doc.subject_kind,
                        doc.subject_id,
                        doc.title,
                        doc.body,
                    ],
                )
                .map_err(map_db_err)?;
            }
            tx.execute(
                "update ingress_receipts
                 set derived_state = 'derived',
                     index_state = 'indexed'
                 where projection_state = 'projected'",
                [],
            )
            .map_err(map_db_err)?;
            Ok(json!({
                "episodes": plan.episodes.len(),
                "insights": plan.insights.len(),
                "verifications": plan.verifications.len(),
                "claims": plan.claims.len(),
                "procedures": plan.procedures.len(),
                "search_docs": plan.search_docs.len(),
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

    fn load_insights(&self) -> Result<Vec<InsightRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select insight_id, episode_id, insight_kind, statement, confidence, scope_json, metadata_json
                 from insights
                 order by insight_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InsightRow {
                    insight_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    insight_kind: row.get(2)?,
                    statement: row.get(3)?,
                    confidence: row.get(4)?,
                    scope_json: parse_json_value(row.get::<_, String>(5)?)?,
                    metadata_json: parse_json_value(row.get::<_, String>(6)?)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_insight_anchors(&self) -> Result<Vec<InsightAnchorRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select insight_id, anchor_id
                 from insight_anchors
                 order by insight_id asc, anchor_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(InsightAnchorRow {
                    insight_id: row.get(0)?,
                    anchor_id: row.get(1)?,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn load_verifications(&self) -> Result<Vec<VerificationRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select verification_id, subject_kind, subject_id, method, status, checked_at, checker, details_json
                 from verifications
                 order by verification_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(VerificationRow {
                    verification_id: row.get(0)?,
                    subject_kind: row.get(1)?,
                    subject_id: row.get(2)?,
                    method: row.get(3)?,
                    status: row.get(4)?,
                    checked_at: row.get(5)?,
                    checker: row.get(6)?,
                    details_json: parse_json_value(row.get::<_, String>(7)?)?,
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

    fn load_procedures(&self) -> Result<Vec<ProcedureRow>> {
        let conn = self.connection()?;
        let mut stmt = conn
            .prepare(
                "select procedure_id, episode_id, title, goal, steps_json, status, confidence, extractor_version, stale
                 from procedures
                 order by procedure_id asc",
            )
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProcedureRow {
                    procedure_id: row.get(0)?,
                    episode_id: row.get(1)?,
                    title: row.get(2)?,
                    goal: row.get(3)?,
                    steps_json: parse_json_value(row.get::<_, String>(4)?)?,
                    status: row.get(5)?,
                    confidence: row.get(6)?,
                    extractor_version: row.get(7)?,
                    stale: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(map_db_err)?;
        rows.map(|row| row.map_err(map_db_err)).collect()
    }

    fn pending_counts(&self) -> Result<(usize, usize, usize)> {
        let conn = self.connection()?;
        Ok((
            count_where(&conn, "ingress_receipts", "projection_state <> 'projected'")?,
            count_where(&conn, "ingress_receipts", "derived_state <> 'derived'")?,
            count_where(&conn, "ingress_receipts", "index_state <> 'indexed'")?,
        ))
    }

    fn doctor_report(&self) -> Result<DoctorReport> {
        let conn = self.connection()?;
        let (pending_projection_count, pending_derived_count, pending_index_count) =
            self.pending_counts()?;
        Ok(DoctorReport {
            db_path: self.db_path.display().to_string(),
            schema_version: axiomsync_domain::KERNEL_SCHEMA_VERSION.to_string(),
            ingress_receipts: count_rows(&conn, "ingress_receipts")?,
            sessions: count_rows(&conn, "sessions")?,
            entries: count_rows(&conn, "entries")?,
            episodes: count_rows(&conn, "episodes")?,
            insights: count_rows(&conn, "insights")?,
            verifications: count_rows(&conn, "verifications")?,
            claims: count_rows(&conn, "claims")?,
            procedures: count_rows(&conn, "procedures")?,
            pending_projection_count,
            pending_derived_count,
            pending_index_count,
        })
    }
}

fn count_rows(conn: &Connection, table: &str) -> Result<usize> {
    conn.query_row(&format!("select count(*) from {table}"), [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|value| value as usize)
    .map_err(map_db_err)
}

fn count_where(conn: &Connection, table: &str, predicate: &str) -> Result<usize> {
    conn.query_row(
        &format!("select count(*) from {table} where {predicate}"),
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value as usize)
    .map_err(map_db_err)
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut stmt = conn
        .prepare(&format!("pragma table_info({table})"))
        .map_err(map_db_err)?;
    let mut rows = stmt.query([]).map_err(map_db_err)?;
    while let Some(row) = rows.next().map_err(map_db_err)? {
        let existing: String = row.get(1).map_err(map_db_err)?;
        if existing == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("alter table {table} add column {column} {definition}"),
        [],
    )
    .map_err(map_db_err)?;
    Ok(())
}

fn upsert_source_cursor_tx(tx: &Transaction<'_>, cursor: &SourceCursorRow) -> Result<()> {
    tx.execute(
        "insert into source_cursor (connector, cursor_key, cursor_value, updated_at, metadata_json)
         values (?1, ?2, ?3, ?4, ?5)
         on conflict(connector, cursor_key) do update set
           cursor_value = excluded.cursor_value,
           updated_at = excluded.updated_at,
           metadata_json = excluded.metadata_json",
        params![
            cursor.connector,
            cursor.cursor_key,
            cursor.cursor_value,
            cursor.updated_at,
            serde_json::to_string(&cursor.metadata_json)?,
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
