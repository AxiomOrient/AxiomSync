use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::stable_id as make_stable_id;
use crate::domain::{
    ArtifactRow, ConvItemRow, ConvSessionRow, ConvTurnRow, DerivePlan, DoctorReport,
    DocumentRecordRow, DocumentView, EpisodeConnectorRow, EpisodeEvidenceSearchRow,
    EpisodeMemberRow, EpisodeRow, EpisodeStatus, EvidenceAnchorRow, ExecutionApprovalRow,
    ExecutionCheckRow, ExecutionEventRow, ExecutionRunRow, ExecutionTaskRow, ExistingRawEventKey,
    ImportJournalRow, IngestPlan, InsightAnchorRow, InsightKind, InsightRow, ItemType,
    ProjectionPlan, PurgePlan, RawEventRow, RepairPlan, ReplayPlan, RunView,
    SearchCommandCandidateRow, SearchDocRedactedRow, SearchEpisodeFtsRow, SelectorType,
    SourceCursorRow, SourceCursorUpsertPlan, TaskView, ThreadItemView, ThreadTurnView, ThreadView,
    VerificationKind, VerificationRow, VerificationStatus, WorkspaceRow,
};
use crate::error::{AxiomError, Result};
use crate::ports::{ReadRepository, TransactionManager, WriteRepository};

mod apply;
mod base;
mod codec;
mod doctor;
mod read_derived;
mod read_ingest;
mod read_projection;
mod traits;
mod views;

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
create unique index if not exists idx_execution_run_stable_id on execution_run(stable_id);
create unique index if not exists idx_execution_task_stable_id on execution_task(stable_id);
create unique index if not exists idx_execution_check_stable_id on execution_check(stable_id);
create unique index if not exists idx_execution_approval_stable_id on execution_approval(stable_id);
create unique index if not exists idx_execution_event_stable_id on execution_event(stable_id);
create unique index if not exists idx_document_record_stable_id on document_record(stable_id);
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

fn add_stable_id_column(conn: &Connection, table: &str) -> Result<()> {
    let exists = conn
        .prepare(&format!("pragma table_info({table})"))
        .map_db_err()?
        .query_map([], |row| row.get::<_, String>(1))
        .map_db_err()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_db_err()?
        .into_iter()
        .any(|column| column == "stable_id");
    if !exists {
        conn.execute(
            &format!("alter table {table} add column stable_id text not null default ''"),
            [],
        )
        .map_db_err()?;
    }
    Ok(())
}

fn stable_id_map(tx: &rusqlite::Transaction<'_>, table: &str) -> Result<HashMap<String, i64>> {
    let mut stmt = tx
        .prepare(&format!("select id, stable_id from {table}"))
        .map_db_err()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_db_err()?;
    let pairs = rows.collect::<rusqlite::Result<Vec<_>>>().map_db_err()?;
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
    let mut stmt = conn
        .prepare("select name from sqlite_master where type = ?1 order by name asc")
        .map_db_err()?;
    let rows = stmt
        .query_map([object_type], |row| row.get::<_, String>(0))
        .map_db_err()?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sqlite_error)
}

fn sqlite_error(error: rusqlite::Error) -> AxiomError {
    AxiomError::Internal(format!("sqlite error: {error}"))
}

trait SqliteResultExt<T> {
    fn map_db_err(self) -> Result<T>;
}

impl<T> SqliteResultExt<T> for rusqlite::Result<T> {
    fn map_db_err(self) -> Result<T> {
        self.map_err(sqlite_error)
    }
}
