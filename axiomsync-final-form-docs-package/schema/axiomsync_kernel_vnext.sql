-- AxiomSync vNext minimal kernel schema
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS source_cursors (
    connector_name TEXT NOT NULL,
    cursor_key TEXT NOT NULL,
    cursor_value TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (connector_name, cursor_key)
);

CREATE TABLE IF NOT EXISTS raw_events (
    raw_event_id TEXT PRIMARY KEY,
    source_kind TEXT NOT NULL,
    connector_name TEXT NOT NULL,
    native_session_id TEXT,
    native_entry_id TEXT,
    event_type TEXT NOT NULL,
    captured_at_ms INTEGER,
    observed_at_ms INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    raw_payload_json TEXT NOT NULL,
    normalized_json TEXT NOT NULL,
    projection_state TEXT NOT NULL DEFAULT 'pending',
    created_at_ms INTEGER NOT NULL,
    UNIQUE (connector_name, dedupe_key)
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    session_kind TEXT NOT NULL,
    stable_key TEXT NOT NULL UNIQUE,
    title TEXT,
    workspace_root TEXT,
    started_at_ms INTEGER,
    ended_at_ms INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS actors (
    actor_id TEXT PRIMARY KEY,
    actor_kind TEXT NOT NULL,
    stable_key TEXT NOT NULL UNIQUE,
    display_name TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS entries (
    entry_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    seq_no INTEGER NOT NULL,
    entry_kind TEXT NOT NULL,
    actor_id TEXT REFERENCES actors(actor_id),
    parent_entry_id TEXT REFERENCES entries(entry_id),
    stable_key TEXT NOT NULL UNIQUE,
    text_body TEXT,
    started_at_ms INTEGER,
    ended_at_ms INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    UNIQUE (session_id, seq_no)
);

CREATE TABLE IF NOT EXISTS entry_raw_events (
    entry_id TEXT NOT NULL REFERENCES entries(entry_id),
    raw_event_id TEXT NOT NULL REFERENCES raw_events(raw_event_id),
    PRIMARY KEY (entry_id, raw_event_id)
);

CREATE TABLE IF NOT EXISTS artifacts (
    artifact_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    entry_id TEXT REFERENCES entries(entry_id),
    artifact_kind TEXT NOT NULL,
    uri TEXT NOT NULL,
    mime_type TEXT,
    sha256 TEXT,
    size_bytes INTEGER,
    stable_key TEXT NOT NULL UNIQUE,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS anchors (
    anchor_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    entry_id TEXT REFERENCES entries(entry_id),
    artifact_id TEXT REFERENCES artifacts(artifact_id),
    anchor_kind TEXT NOT NULL,
    locator_json TEXT NOT NULL,
    preview_text TEXT,
    fingerprint TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    UNIQUE (session_id, fingerprint)
);

CREATE TABLE IF NOT EXISTS episodes (
    episode_id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(session_id),
    episode_kind TEXT NOT NULL,
    title TEXT NOT NULL,
    summary_text TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at_ms INTEGER,
    ended_at_ms INTEGER,
    stable_key TEXT UNIQUE,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS episode_anchors (
    episode_id TEXT NOT NULL REFERENCES episodes(episode_id),
    anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id),
    role TEXT NOT NULL,
    PRIMARY KEY (episode_id, anchor_id)
);

CREATE TABLE IF NOT EXISTS insights (
    insight_id TEXT PRIMARY KEY,
    episode_id TEXT REFERENCES episodes(episode_id),
    insight_kind TEXT NOT NULL,
    statement TEXT NOT NULL,
    scope_json TEXT NOT NULL DEFAULT '{}',
    confidence REAL NOT NULL DEFAULT 0.5,
    stable_key TEXT UNIQUE,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS insight_anchors (
    insight_id TEXT NOT NULL REFERENCES insights(insight_id),
    anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id),
    PRIMARY KEY (insight_id, anchor_id)
);

CREATE TABLE IF NOT EXISTS verifications (
    verification_id TEXT PRIMARY KEY,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    method TEXT NOT NULL,
    status TEXT NOT NULL,
    checked_at_ms INTEGER NOT NULL,
    checker TEXT,
    details_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS procedures (
    procedure_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    purpose TEXT,
    preconditions_json TEXT NOT NULL DEFAULT '[]',
    expected_outcome TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    stable_key TEXT UNIQUE,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS procedure_steps (
    procedure_id TEXT NOT NULL REFERENCES procedures(procedure_id),
    step_no INTEGER NOT NULL,
    action_text TEXT NOT NULL,
    evidence_expectation TEXT,
    PRIMARY KEY (procedure_id, step_no)
);

CREATE TABLE IF NOT EXISTS procedure_anchors (
    procedure_id TEXT NOT NULL REFERENCES procedures(procedure_id),
    anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id),
    PRIMARY KEY (procedure_id, anchor_id)
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

-- Rebuildable index; create in migration/runtime when FTS5 available.
-- CREATE VIRTUAL TABLE search_docs_fts USING fts5(doc_id UNINDEXED, title, body, tokenize='unicode61');
