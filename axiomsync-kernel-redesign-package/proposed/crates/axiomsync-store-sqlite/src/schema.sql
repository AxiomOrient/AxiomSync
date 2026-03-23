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
  content_hash TEXT,
  dedupe_key TEXT,
  payload_json TEXT NOT NULL,
  raw_payload_json TEXT,
  inserted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ingress_receipts_dedupe
  ON ingress_receipts(dedupe_key)
  WHERE dedupe_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  session_kind TEXT NOT NULL,
  external_session_key TEXT,
  title TEXT,
  workspace_root TEXT,
  opened_at TEXT,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_external
  ON sessions(session_kind, external_session_key)
  WHERE external_session_key IS NOT NULL;

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
  actor_id TEXT REFERENCES actors(actor_id),
  parent_entry_id TEXT REFERENCES entries(entry_id),
  external_entry_key TEXT,
  text_body TEXT,
  started_at TEXT,
  ended_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_session_seq
  ON entries(session_id, seq_no);

CREATE TABLE IF NOT EXISTS artifacts (
  artifact_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
  entry_id TEXT REFERENCES entries(entry_id) ON DELETE SET NULL,
  artifact_kind TEXT NOT NULL,
  uri TEXT NOT NULL,
  mime_type TEXT,
  sha256 TEXT,
  size_bytes INTEGER,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS anchors (
  anchor_id TEXT PRIMARY KEY,
  entry_id TEXT REFERENCES entries(entry_id) ON DELETE CASCADE,
  artifact_id TEXT REFERENCES artifacts(artifact_id) ON DELETE CASCADE,
  anchor_kind TEXT NOT NULL,
  locator_json TEXT NOT NULL,
  preview_text TEXT,
  fingerprint TEXT
);

CREATE TABLE IF NOT EXISTS episodes (
  episode_id TEXT PRIMARY KEY,
  session_id TEXT REFERENCES sessions(session_id) ON DELETE SET NULL,
  episode_kind TEXT NOT NULL,
  summary TEXT NOT NULL,
  status TEXT,
  confidence REAL NOT NULL DEFAULT 0.0,
  extractor_version TEXT NOT NULL,
  stale INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS claims (
  claim_id TEXT PRIMARY KEY,
  episode_id TEXT REFERENCES episodes(episode_id) ON DELETE CASCADE,
  claim_kind TEXT NOT NULL,
  statement TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 0.0,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS claim_evidence (
  claim_id TEXT NOT NULL REFERENCES claims(claim_id) ON DELETE CASCADE,
  anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id) ON DELETE CASCADE,
  support_kind TEXT NOT NULL,
  PRIMARY KEY (claim_id, anchor_id)
);

CREATE TABLE IF NOT EXISTS procedures (
  procedure_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  goal TEXT,
  steps_json TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 0.0,
  extractor_version TEXT NOT NULL,
  stale INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS procedure_evidence (
  procedure_id TEXT NOT NULL REFERENCES procedures(procedure_id) ON DELETE CASCADE,
  anchor_id TEXT NOT NULL REFERENCES anchors(anchor_id) ON DELETE CASCADE,
  support_kind TEXT NOT NULL,
  PRIMARY KEY (procedure_id, anchor_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS entry_search_fts USING fts5(
  entry_id UNINDEXED,
  session_kind UNINDEXED,
  connector UNINDEXED,
  text_body
);

CREATE VIRTUAL TABLE IF NOT EXISTS episode_search_fts USING fts5(
  episode_id UNINDEXED,
  episode_kind UNINDEXED,
  summary
);

CREATE VIRTUAL TABLE IF NOT EXISTS procedure_search_fts USING fts5(
  procedure_id UNINDEXED,
  title,
  goal,
  steps_text
);
