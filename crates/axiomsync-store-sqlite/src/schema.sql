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

CREATE UNIQUE INDEX IF NOT EXISTS idx_ingress_receipts_dedupe
  ON ingress_receipts(dedupe_key)
  WHERE dedupe_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_ingress_receipts_projection_state
  ON ingress_receipts(projection_state);

CREATE INDEX IF NOT EXISTS idx_ingress_receipts_derived_state
  ON ingress_receipts(derived_state);

CREATE INDEX IF NOT EXISTS idx_ingress_receipts_index_state
  ON ingress_receipts(index_state);

CREATE INDEX IF NOT EXISTS idx_ingress_receipts_observed_at
  ON ingress_receipts(observed_at, receipt_id);

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

CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_external
  ON sessions(session_kind, connector, external_session_key)
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
  actor_id TEXT REFERENCES actors(actor_id) ON DELETE SET NULL,
  parent_entry_id TEXT REFERENCES entries(entry_id) ON DELETE SET NULL,
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

CREATE TABLE IF NOT EXISTS insights (
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
  episode_id TEXT REFERENCES episodes(episode_id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  goal TEXT,
  steps_json TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
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
  session_id UNINDEXED,
  entry_kind UNINDEXED,
  text_body
);

CREATE VIRTUAL TABLE IF NOT EXISTS episode_search_fts USING fts5(
  episode_id UNINDEXED,
  episode_kind UNINDEXED,
  summary
);

CREATE VIRTUAL TABLE IF NOT EXISTS claim_search_fts USING fts5(
  claim_id UNINDEXED,
  claim_kind UNINDEXED,
  statement
);

CREATE VIRTUAL TABLE IF NOT EXISTS insight_search_fts USING fts5(
  insight_id UNINDEXED,
  insight_kind UNINDEXED,
  statement
);

CREATE VIRTUAL TABLE IF NOT EXISTS procedure_search_fts USING fts5(
  procedure_id UNINDEXED,
  title,
  goal,
  steps_text
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
);
