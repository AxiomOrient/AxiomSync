PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS ingress_receipts (
  receipt_id TEXT PRIMARY KEY,
  batch_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  connector TEXT NOT NULL,
  session_kind TEXT NOT NULL,
  external_session_key TEXT,
  external_entry_key TEXT,
  event_kind TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  captured_at TEXT,
  workspace_root TEXT,
  content_hash TEXT NOT NULL,
  dedupe_key TEXT,
  payload_json TEXT NOT NULL,
  raw_payload_json TEXT,
  artifacts_json TEXT NOT NULL DEFAULT '[]',
  normalized_json TEXT NOT NULL DEFAULT '{}',
  projection_state TEXT NOT NULL DEFAULT 'pending',
  derived_state TEXT NOT NULL DEFAULT 'pending',
  index_state TEXT NOT NULL DEFAULT 'pending',
  inserted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS source_cursor (
  connector TEXT NOT NULL,
  cursor_key TEXT NOT NULL,
  cursor_value TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (connector, cursor_key)
);

CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  session_kind TEXT NOT NULL,
  connector TEXT NOT NULL,
  external_session_key TEXT,
  title TEXT,
  workspace_root TEXT,
  opened_at TEXT,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS actors (
  actor_id TEXT PRIMARY KEY,
  actor_kind TEXT NOT NULL,
  stable_key TEXT,
  display_name TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS entries (
  entry_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
  seq_no INTEGER NOT NULL,
  entry_kind TEXT NOT NULL,
  actor_id TEXT REFERENCES actors(actor_id) ON DELETE SET NULL,
  parent_entry_id TEXT REFERENCES entries(entry_id) ON DELETE SET NULL,
  external_entry_key TEXT,
  text_body TEXT,
  started_at TEXT,
  ended_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);
