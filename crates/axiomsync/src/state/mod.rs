use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::models::{MigrationRunRecord, RepairRunRecord, TraceIndexEntry};

mod capabilities;
mod db;
mod events;
mod links;
mod om;
mod promotion_checkpoint;
mod queue;
mod queue_lane;
mod resources;
pub(crate) mod schema;
mod search;
mod sessions;

pub use capabilities::{
    ContextStore, EventStore, LinkStore, ResourceStore, SearchProjectionStore, SessionStore,
};
pub(crate) use om::{OmActiveEntry, OmContinuationHints};
pub(crate) use promotion_checkpoint::PromotionCheckpointPhase;

#[derive(Clone)]
pub struct SqliteStateStore {
    db_path: Arc<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmReflectionApplyOutcome {
    Applied,
    StaleGeneration,
    IdempotentEvent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OmReflectionApplyContext<'a> {
    pub current_task: Option<&'a str>,
    pub suggested_response: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmReflectionBufferPayload<'a> {
    pub reflection: &'a str,
    pub reflection_token_count: u32,
    pub reflection_input_tokens: u32,
}

impl std::fmt::Debug for SqliteStateStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStateStore").finish_non_exhaustive()
    }
}

impl SqliteStateStore {
    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.open_connection()?;
        f(&conn)
    }

    fn with_tx<T>(&self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut conn = self.open_connection()?;
        let tx = conn.transaction()?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: Arc::new(path.to_path_buf()),
        };
        store.ensure_schema()?;
        #[cfg(unix)]
        harden_sqlite_permissions(path)?;
        Ok(store)
    }

    fn open_connection(&self) -> Result<Connection> {
        let conn = Connection::open(self.db_path.as_ref())?;
        configure_connection(&conn)?;
        #[cfg(unix)]
        harden_sqlite_permissions(self.db_path.as_ref())?;
        Ok(conn)
    }

    #[cfg(test)]
    pub(crate) fn test_connection(&self) -> Result<Connection> {
        self.open_connection()
    }

    pub fn get_system_value(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let value = conn
                .query_row(
                    "SELECT value FROM system_kv WHERE key = ?1",
                    params![key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(value)
        })
    }

    pub fn set_system_value(&self, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO system_kv(key, value, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(key) DO UPDATE SET
                  value = excluded.value,
                  updated_at = excluded.updated_at
                ",
                params![key, value, Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }

    pub fn upsert_index_state(
        &self,
        uri: &str,
        content_hash: &str,
        mtime: i64,
        status: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO index_state(uri, content_hash, mtime, indexed_at, status)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(uri) DO UPDATE SET
                  content_hash=excluded.content_hash,
                  mtime=excluded.mtime,
                  indexed_at=excluded.indexed_at,
                  status=excluded.status
                ",
                params![uri, content_hash, mtime, now, status],
            )?;
            Ok(())
        })
    }

    pub fn get_index_state_hash(&self, uri: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let value = conn
                .query_row(
                    "SELECT content_hash FROM index_state WHERE uri = ?1",
                    params![uri],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(value)
        })
    }

    pub fn get_index_state(&self, uri: &str) -> Result<Option<(String, i64)>> {
        self.with_conn(|conn| {
            let value = conn
                .query_row(
                    "SELECT content_hash, mtime FROM index_state WHERE uri = ?1",
                    params![uri],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            Ok(value)
        })
    }

    pub fn list_index_state_uris(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT uri FROM index_state ORDER BY uri ASC")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn list_index_state_entries(&self) -> Result<Vec<(String, i64)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT uri, mtime FROM index_state ORDER BY uri ASC")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn remove_index_state(&self, uri: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let affected = conn.execute("DELETE FROM index_state WHERE uri = ?1", params![uri])?;
            Ok(affected > 0)
        })
    }

    pub fn remove_index_state_with_prefix(&self, uri_prefix: &str) -> Result<usize> {
        self.with_conn(|conn| {
            let escaped_prefix = escape_sql_like_pattern(uri_prefix);
            let affected = conn.execute(
                "DELETE FROM index_state WHERE uri = ?1 OR uri LIKE ?2 ESCAPE '\\'",
                params![uri_prefix, format!("{escaped_prefix}/%")],
            )?;
            Ok(affected)
        })
    }

    pub fn clear_index_state(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM index_state", [])?;
            Ok(())
        })
    }

    fn count_table(&self, table: &'static str) -> Result<usize> {
        self.with_conn(|conn| {
            let count = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })?;
            Ok(usize::try_from(count).unwrap_or(usize::MAX))
        })
    }

    pub fn search_document_count(&self) -> Result<usize> {
        self.count_table("search_docs")
    }

    pub fn event_count(&self) -> Result<usize> {
        self.count_table("events")
    }

    pub fn link_count(&self) -> Result<usize> {
        self.count_table("links")
    }

    pub fn trace_count(&self) -> Result<usize> {
        self.count_table("trace_index")
    }

    pub fn fts_ready(&self) -> Result<bool> {
        self.with_conn(|conn| {
            let ready = conn
                .query_row(
                    "SELECT value FROM system_kv WHERE key = 'search_docs_fts_schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(ready.is_some())
        })
    }

    pub fn record_migration_run(&self, run: &MigrationRunRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO schema_migration_runs(run_id, operation, started_at, finished_at, status, details_json)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(run_id) DO UPDATE SET
                  operation = excluded.operation,
                  started_at = excluded.started_at,
                  finished_at = excluded.finished_at,
                  status = excluded.status,
                  details_json = excluded.details_json
                ",
                params![
                    run.run_id,
                    run.operation,
                    run.started_at,
                    run.finished_at,
                    run.status,
                    run.details.as_ref().map(serde_json::to_string).transpose()?,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_migration_runs(&self, limit: usize) -> Result<Vec<MigrationRunRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r"
                SELECT run_id, operation, started_at, finished_at, status, details_json
                FROM schema_migration_runs
                ORDER BY started_at DESC, run_id DESC
                LIMIT ?1
                ",
            )?;
            let rows = stmt.query_map(params![usize_to_i64_saturating(limit)], |row| {
                let details = row
                    .get::<_, Option<String>>(5)?
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?;
                Ok(MigrationRunRecord {
                    run_id: row.get(0)?,
                    operation: row.get(1)?,
                    started_at: row.get(2)?,
                    finished_at: row.get(3)?,
                    status: row.get(4)?,
                    details,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }

    pub fn record_repair_run(&self, run: &RepairRunRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO repair_runs(run_id, repair_type, started_at, finished_at, status, details_json)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(run_id) DO UPDATE SET
                  repair_type = excluded.repair_type,
                  started_at = excluded.started_at,
                  finished_at = excluded.finished_at,
                  status = excluded.status,
                  details_json = excluded.details_json
                ",
                params![
                    run.run_id,
                    run.repair_type,
                    run.started_at,
                    run.finished_at,
                    run.status,
                    run.details.as_ref().map(serde_json::to_string).transpose()?,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_repair_runs(&self, limit: usize) -> Result<Vec<RepairRunRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r"
                SELECT run_id, repair_type, started_at, finished_at, status, details_json
                FROM repair_runs
                ORDER BY started_at DESC, run_id DESC
                LIMIT ?1
                ",
            )?;
            let rows = stmt.query_map(params![usize_to_i64_saturating(limit)], |row| {
                let details = row
                    .get::<_, Option<String>>(5)?
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?;
                Ok(RepairRunRecord {
                    run_id: row.get(0)?,
                    repair_type: row.get(1)?,
                    started_at: row.get(2)?,
                    finished_at: row.get(3)?,
                    status: row.get(4)?,
                    details,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }

    pub fn upsert_trace_index(&self, entry: &TraceIndexEntry) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r"
                INSERT INTO trace_index(trace_id, uri, request_type, query, target_uri, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(trace_id) DO UPDATE SET
                  uri=excluded.uri,
                  request_type=excluded.request_type,
                  query=excluded.query,
                  target_uri=excluded.target_uri,
                  created_at=excluded.created_at
                ",
                params![
                    entry.trace_id,
                    entry.uri,
                    entry.request_type,
                    entry.query,
                    entry.target_uri,
                    entry.created_at
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_trace_index(&self, trace_id: &str) -> Result<Option<TraceIndexEntry>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    r"
                    SELECT trace_id, uri, request_type, query, target_uri, created_at
                    FROM trace_index
                    WHERE trace_id = ?1
                    ",
                    params![trace_id],
                    |row| {
                        Ok(TraceIndexEntry {
                            trace_id: row.get(0)?,
                            uri: row.get(1)?,
                            request_type: row.get(2)?,
                            query: row.get(3)?,
                            target_uri: row.get(4)?,
                            created_at: row.get(5)?,
                        })
                    },
                )
                .optional()?;

            Ok(row)
        })
    }

    pub fn list_trace_index(&self, limit: usize) -> Result<Vec<TraceIndexEntry>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                r"
                SELECT trace_id, uri, request_type, query, target_uri, created_at
                FROM trace_index
                ORDER BY created_at DESC, trace_id ASC
                LIMIT ?1
                ",
            )?;
            let rows = stmt.query_map(params![usize_to_i64_saturating(limit)], |row| {
                Ok(TraceIndexEntry {
                    trace_id: row.get(0)?,
                    uri: row.get(1)?,
                    request_type: row.get(2)?,
                    query: row.get(3)?,
                    target_uri: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }
}

fn escape_sql_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub(super) fn usize_to_i64_saturating(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_millis(5_000))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

#[cfg(unix)]
fn harden_sqlite_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    for suffix in ["", "-wal", "-shm"] {
        let mut os = path.as_os_str().to_os_string();
        os.push(suffix);
        let candidate = PathBuf::from(os);
        if candidate.exists() {
            std::fs::set_permissions(candidate, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
