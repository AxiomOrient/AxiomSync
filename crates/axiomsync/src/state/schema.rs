use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AxiomError, Result};
use crate::models::{QueueEventStatus, ReconcileRunStatus};

use super::SqliteStateStore;

pub(crate) const SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY: &str = "search_docs_fts_schema_version";
pub(crate) const SEARCH_DOCS_FTS_SCHEMA_VERSION: &str = "fts5-v1";
pub(crate) const CONTEXT_SCHEMA_VERSION_KEY: &str = "context_schema_version";
pub(crate) const CONTEXT_SCHEMA_VERSION: &str = "v3";
pub(crate) const RELEASE_CONTRACT_VERSION_KEY: &str = "release_contract_version";
pub(crate) const RELEASE_CONTRACT_VERSION: &str = "v1";
pub(crate) const INDEX_PROFILE_STAMP_KEY: &str = "index_profile_stamp";
pub(crate) const RUNTIME_RESTORE_SOURCE_KEY: &str = "runtime_restore_source";

const STATE_SCHEMA_SQL: &str = r"
    PRAGMA journal_mode = WAL;
    PRAGMA foreign_keys = ON;
    CREATE TABLE IF NOT EXISTS index_state (
        uri TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        mtime INTEGER NOT NULL,
        indexed_at TEXT NOT NULL,
        status TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS outbox (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        event_type TEXT NOT NULL,
        uri TEXT NOT NULL,
        payload_json TEXT NOT NULL,
        created_at TEXT NOT NULL,
        attempt_count INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL CHECK(status IN ('new', 'processing', 'done', 'dead_letter')),
        next_attempt_at TEXT NOT NULL,
        lane TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS queue_checkpoint (
        worker_name TEXT PRIMARY KEY,
        last_event_id INTEGER NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS reconcile_runs (
        run_id TEXT PRIMARY KEY,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        drift_count INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL CHECK(status IN ('running', 'dry_run', 'success', 'failed'))
    );

    CREATE TABLE IF NOT EXISTS trace_index (
        trace_id TEXT PRIMARY KEY,
        uri TEXT NOT NULL,
        request_type TEXT NOT NULL,
        query TEXT NOT NULL,
        target_uri TEXT,
        created_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_trace_index_created_at
    ON trace_index(created_at DESC);

    CREATE TABLE IF NOT EXISTS system_kv (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS schema_migration_runs (
        run_id TEXT PRIMARY KEY,
        operation TEXT NOT NULL,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        status TEXT NOT NULL,
        details_json TEXT
    );

    CREATE TABLE IF NOT EXISTS repair_runs (
        run_id TEXT PRIMARY KEY,
        repair_type TEXT NOT NULL,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        status TEXT NOT NULL,
        details_json TEXT
    );

    CREATE TABLE IF NOT EXISTS search_docs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uri TEXT NOT NULL UNIQUE,
        parent_uri TEXT,
        is_leaf INTEGER NOT NULL,
        context_type TEXT NOT NULL,
        name TEXT NOT NULL,
        abstract_text TEXT NOT NULL,
        content TEXT NOT NULL,
        tags_text TEXT NOT NULL,
        mime TEXT,
        namespace TEXT,
        kind TEXT,
        event_time INTEGER,
        updated_at TEXT NOT NULL,
        depth INTEGER NOT NULL,
        source_weight REAL NOT NULL DEFAULT 0.0,
        freshness_bucket INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS search_doc_tags (
        doc_id INTEGER NOT NULL,
        tag TEXT NOT NULL,
        PRIMARY KEY (doc_id, tag),
        FOREIGN KEY (doc_id) REFERENCES search_docs(id) ON DELETE CASCADE
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS search_docs_fts
    USING fts5(
        uri UNINDEXED,
        name,
        abstract_text,
        content,
        tags_text,
        content='search_docs',
        content_rowid='id',
        tokenize='unicode61'
    );

    CREATE TRIGGER IF NOT EXISTS search_docs_ai AFTER INSERT ON search_docs BEGIN
        INSERT INTO search_docs_fts(rowid, uri, name, abstract_text, content, tags_text)
        VALUES (new.id, new.uri, new.name, new.abstract_text, new.content, new.tags_text);
    END;

    CREATE TRIGGER IF NOT EXISTS search_docs_ad AFTER DELETE ON search_docs BEGIN
        INSERT INTO search_docs_fts(search_docs_fts, rowid, uri, name, abstract_text, content, tags_text)
        VALUES ('delete', old.id, old.uri, old.name, old.abstract_text, old.content, old.tags_text);
    END;

    CREATE TRIGGER IF NOT EXISTS search_docs_au AFTER UPDATE ON search_docs BEGIN
        INSERT INTO search_docs_fts(search_docs_fts, rowid, uri, name, abstract_text, content, tags_text)
        VALUES ('delete', old.id, old.uri, old.name, old.abstract_text, old.content, old.tags_text);
        INSERT INTO search_docs_fts(rowid, uri, name, abstract_text, content, tags_text)
        VALUES (new.id, new.uri, new.name, new.abstract_text, new.content, new.tags_text);
    END;

    CREATE TABLE IF NOT EXISTS om_records (
        id TEXT PRIMARY KEY,
        scope TEXT NOT NULL CHECK(scope IN ('session', 'thread', 'resource')),
        scope_key TEXT NOT NULL UNIQUE,
        session_id TEXT,
        thread_id TEXT,
        resource_id TEXT,
        generation_count INTEGER NOT NULL DEFAULT 0,
        last_applied_outbox_event_id INTEGER,
        origin_type TEXT NOT NULL CHECK(origin_type IN ('initial', 'reflection')),
        active_observations TEXT NOT NULL DEFAULT '',
        observation_token_count INTEGER NOT NULL DEFAULT 0,
        pending_message_tokens INTEGER NOT NULL DEFAULT 0,
        last_observed_at TEXT,
        current_task TEXT,
        suggested_response TEXT,
        last_activated_message_ids_json TEXT NOT NULL DEFAULT '[]',
        observer_trigger_count_total INTEGER NOT NULL DEFAULT 0,
        reflector_trigger_count_total INTEGER NOT NULL DEFAULT 0,
        is_observing INTEGER NOT NULL DEFAULT 0,
        is_reflecting INTEGER NOT NULL DEFAULT 0,
        is_buffering_observation INTEGER NOT NULL DEFAULT 0,
        is_buffering_reflection INTEGER NOT NULL DEFAULT 0,
        last_buffered_at_tokens INTEGER NOT NULL DEFAULT 0,
        last_buffered_at_time TEXT,
        buffered_reflection TEXT,
        buffered_reflection_tokens INTEGER,
        buffered_reflection_input_tokens INTEGER,
        reflected_observation_line_count INTEGER,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS om_observation_chunks (
        id TEXT PRIMARY KEY,
        record_id TEXT NOT NULL,
        seq INTEGER NOT NULL,
        cycle_id TEXT NOT NULL,
        observations TEXT NOT NULL,
        token_count INTEGER NOT NULL,
        message_tokens INTEGER NOT NULL,
        message_ids_json TEXT NOT NULL,
        last_observed_at TEXT NOT NULL,
        created_at TEXT NOT NULL,
        FOREIGN KEY (record_id) REFERENCES om_records(id) ON DELETE CASCADE,
        UNIQUE(record_id, seq)
    );

    CREATE TABLE IF NOT EXISTS om_observer_applied_events (
        outbox_event_id INTEGER PRIMARY KEY,
        scope_key TEXT NOT NULL,
        generation_count INTEGER NOT NULL,
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS om_scope_sessions (
        scope_key TEXT NOT NULL,
        session_id TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY(scope_key, session_id)
    );

    CREATE TABLE IF NOT EXISTS om_thread_states (
        scope_key TEXT NOT NULL,
        thread_id TEXT NOT NULL,
        last_observed_at TEXT,
        current_task TEXT,
        suggested_response TEXT,
        updated_at TEXT NOT NULL,
        PRIMARY KEY(scope_key, thread_id)
    );

    CREATE TABLE IF NOT EXISTS om_entries (
        entry_id TEXT PRIMARY KEY,
        scope_key TEXT NOT NULL,
        canonical_thread_id TEXT NOT NULL,
        priority TEXT NOT NULL CHECK(priority IN ('high', 'medium', 'low')),
        text TEXT NOT NULL,
        source_message_ids_json TEXT NOT NULL,
        origin_kind TEXT NOT NULL CHECK(origin_kind IN ('observation', 'reflection')),
        created_at TEXT NOT NULL,
        superseded_by TEXT
    );

    CREATE TABLE IF NOT EXISTS om_reflection_events (
        event_id TEXT PRIMARY KEY,
        scope_key TEXT NOT NULL,
        covers_entry_ids_json TEXT NOT NULL,
        reflection_entry_id TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS om_continuation_state (
        scope_key TEXT NOT NULL,
        canonical_thread_id TEXT NOT NULL,
        current_task TEXT,
        suggested_response TEXT,
        confidence REAL NOT NULL DEFAULT 0.0,
        source_kind TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY(scope_key, canonical_thread_id)
    );

    CREATE TABLE IF NOT EXISTS om_runtime_metrics (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        reflect_apply_attempts_total INTEGER NOT NULL DEFAULT 0,
        reflect_apply_applied_total INTEGER NOT NULL DEFAULT 0,
        reflect_apply_stale_generation_total INTEGER NOT NULL DEFAULT 0,
        reflect_apply_idempotent_total INTEGER NOT NULL DEFAULT 0,
        reflect_apply_latency_ms_total INTEGER NOT NULL DEFAULT 0,
        reflect_apply_latency_ms_max INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS memory_promotion_checkpoints (
        session_id TEXT NOT NULL,
        checkpoint_id TEXT NOT NULL,
        request_hash TEXT NOT NULL,
        request_json TEXT NOT NULL,
        phase TEXT NOT NULL CHECK(phase IN ('pending', 'applying', 'applied')),
        result_json TEXT,
        applied_at TEXT,
        attempt_count INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (session_id, checkpoint_id)
    );

    CREATE INDEX IF NOT EXISTS idx_search_docs_uri ON search_docs(uri);
    CREATE INDEX IF NOT EXISTS idx_search_docs_parent_uri ON search_docs(parent_uri);
    CREATE INDEX IF NOT EXISTS idx_search_docs_restore_order ON search_docs(depth ASC, uri ASC);
    CREATE INDEX IF NOT EXISTS idx_search_docs_mime ON search_docs(mime);
    CREATE INDEX IF NOT EXISTS idx_search_docs_namespace_kind_time
    ON search_docs(namespace, kind, event_time DESC);
    CREATE INDEX IF NOT EXISTS idx_search_doc_tags_tag ON search_doc_tags(tag);
    CREATE INDEX IF NOT EXISTS idx_om_records_updated_at ON om_records(updated_at);
    CREATE INDEX IF NOT EXISTS idx_om_records_scope_session ON om_records(scope, session_id);
    CREATE INDEX IF NOT EXISTS idx_om_chunks_record_created_at ON om_observation_chunks(record_id, created_at);
    CREATE INDEX IF NOT EXISTS idx_om_observer_applied_scope_generation
    ON om_observer_applied_events(scope_key, generation_count, outbox_event_id);
    CREATE INDEX IF NOT EXISTS idx_om_scope_sessions_scope_updated_at
    ON om_scope_sessions(scope_key, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_om_thread_states_scope_updated_at
    ON om_thread_states(scope_key, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_om_entries_scope_created_at
    ON om_entries(scope_key, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_om_entries_scope_thread
    ON om_entries(scope_key, canonical_thread_id);
    CREATE INDEX IF NOT EXISTS idx_om_continuation_scope_updated_at
    ON om_continuation_state(scope_key, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_om_reflection_events_scope_created_at
    ON om_reflection_events(scope_key, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_memory_promotion_checkpoints_session
    ON memory_promotion_checkpoints(session_id, updated_at DESC);
";

impl SqliteStateStore {
    pub fn ensure_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(STATE_SCHEMA_SQL)?;
        normalize_status_column(&conn, "outbox", "status")?;
        validate_status_domain(
            &conn,
            "outbox",
            "status",
            &outbox_status_domain_values(),
            "unsupported outbox schema",
        )?;
        normalize_status_column(&conn, "reconcile_runs", "status")?;
        validate_status_domain(
            &conn,
            "reconcile_runs",
            "status",
            &reconcile_run_status_domain_values(),
            "unsupported reconcile_runs schema",
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_outbox_status_next_attempt_id ON outbox(status, next_attempt_at, id)",
            [],
        )?;
        ensure_context_schema(&conn)?;
        ensure_search_docs_fts_bootstrapped(&conn)?;
        ensure_release_contract_version(&conn)?;
        Ok(())
    }
}

fn outbox_status_domain_values() -> [&'static str; 4] {
    [
        QueueEventStatus::New.as_str(),
        QueueEventStatus::Processing.as_str(),
        QueueEventStatus::Done.as_str(),
        QueueEventStatus::DeadLetter.as_str(),
    ]
}

fn reconcile_run_status_domain_values() -> [&'static str; 4] {
    [
        ReconcileRunStatus::Running.as_str(),
        ReconcileRunStatus::DryRun.as_str(),
        ReconcileRunStatus::Success.as_str(),
        ReconcileRunStatus::Failed.as_str(),
    ]
}

fn normalize_status_column(conn: &Connection, table: &str, column: &str) -> Result<()> {
    conn.execute(
        &format!(
            "UPDATE {table} SET {column} = lower(trim({column})) WHERE {column} <> lower(trim({column}))"
        ),
        [],
    )?;
    Ok(())
}

fn validate_status_domain(
    conn: &Connection,
    table: &str,
    column: &str,
    allowed: &[&str],
    error_prefix: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("SELECT DISTINCT {column} FROM {table}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut invalid = Vec::<String>::new();
    for row in rows {
        let value = row?.trim().to_ascii_lowercase();
        if !allowed.contains(&value.as_str()) {
            invalid.push(value);
        }
    }

    if invalid.is_empty() {
        return Ok(());
    }

    invalid.sort();
    invalid.dedup();
    Err(AxiomError::Validation(format!(
        "{error_prefix}: invalid status value(s): {}",
        invalid.join(", ")
    )))
}

fn ensure_search_docs_fts_bootstrapped(conn: &Connection) -> Result<()> {
    let marker = conn
        .query_row(
            "SELECT value FROM system_kv WHERE key = ?1",
            params![SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if marker.as_deref() == Some(SEARCH_DOCS_FTS_SCHEMA_VERSION) {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO search_docs_fts(search_docs_fts) VALUES ('rebuild')",
        [],
    )?;
    set_kv_on_conn(
        conn,
        SEARCH_DOCS_FTS_SCHEMA_VERSION_KEY,
        SEARCH_DOCS_FTS_SCHEMA_VERSION,
    )?;
    Ok(())
}

fn ensure_context_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS resources (
            resource_id TEXT PRIMARY KEY,
            uri TEXT NOT NULL UNIQUE,
            namespace TEXT NOT NULL,
            kind TEXT NOT NULL,
            title TEXT,
            mime TEXT,
            tags_json TEXT NOT NULL,
            attrs_json TEXT NOT NULL,
            object_uri TEXT,
            excerpt_text TEXT,
            content_hash TEXT NOT NULL,
            tombstoned_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY,
            uri TEXT NOT NULL UNIQUE,
            namespace TEXT NOT NULL,
            kind TEXT NOT NULL,
            event_time INTEGER NOT NULL,
            title TEXT,
            summary_text TEXT,
            severity TEXT,
            actor_uri TEXT,
            subject_uri TEXT,
            run_id TEXT,
            session_id TEXT,
            tags_json TEXT NOT NULL,
            attrs_json TEXT NOT NULL,
            object_uri TEXT,
            content_hash TEXT,
            tombstoned_at INTEGER,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            uri TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS links (
            link_id TEXT PRIMARY KEY,
            namespace TEXT NOT NULL,
            from_uri TEXT NOT NULL,
            relation TEXT NOT NULL,
            to_uri TEXT NOT NULL,
            weight REAL NOT NULL,
            attrs_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(namespace, from_uri, relation, to_uri)
        );

        CREATE INDEX IF NOT EXISTS idx_resources_namespace_kind_updated
        ON resources(namespace, kind, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_resources_uri ON resources(uri);
        CREATE INDEX IF NOT EXISTS idx_resources_tombstoned_at ON resources(tombstoned_at);

        CREATE INDEX IF NOT EXISTS idx_events_namespace_kind_time
        ON events(namespace, kind, event_time DESC);
        CREATE INDEX IF NOT EXISTS idx_events_time ON events(event_time DESC);
        CREATE INDEX IF NOT EXISTS idx_events_run_id ON events(run_id);
        CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);
        CREATE INDEX IF NOT EXISTS idx_events_tombstoned_at ON events(tombstoned_at);

        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_links_namespace_relation_created
        ON links(namespace, relation, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_links_from_uri ON links(from_uri, relation, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_links_to_uri ON links(to_uri, relation, created_at DESC);
        ",
    )?;

    set_kv_on_conn(conn, CONTEXT_SCHEMA_VERSION_KEY, CONTEXT_SCHEMA_VERSION)?;
    Ok(())
}

fn ensure_release_contract_version(conn: &Connection) -> Result<()> {
    set_kv_on_conn(conn, RELEASE_CONTRACT_VERSION_KEY, RELEASE_CONTRACT_VERSION)
}

fn set_kv_on_conn(conn: &Connection, key: &str, value: &str) -> Result<()> {
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
}
