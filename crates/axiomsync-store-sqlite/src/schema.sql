create table if not exists workspace (
  id integer primary key autoincrement,
  stable_id text not null default '',
  canonical_root text not null,
  repo_remote text,
  branch text,
  worktree_path text
);

create table if not exists raw_event (
  id integer primary key autoincrement,
  stable_id text not null default '',
  connector text not null,
  native_schema_version text,
  native_session_id text not null,
  native_event_id text,
  event_type text not null,
  ts_ms integer not null,
  payload_json text not null,
  payload_sha256 blob not null
);

create table if not exists source_cursor (
  connector text not null,
  cursor_key text not null,
  cursor_value text not null,
  updated_at_ms integer not null,
  primary key (connector, cursor_key)
);

create table if not exists import_journal (
  id integer primary key autoincrement,
  stable_id text not null default '',
  connector text not null,
  imported_events integer not null,
  skipped_events integer not null,
  cursor_key text,
  cursor_value text,
  applied_at_ms integer not null
);

create table if not exists conv_session (
  id integer primary key autoincrement,
  stable_id text not null default '',
  connector text not null,
  native_session_id text not null,
  workspace_id integer references workspace(id) on delete set null,
  title text,
  transcript_uri text,
  status text not null,
  started_at_ms integer not null,
  ended_at_ms integer
);

create table if not exists conv_turn (
  id integer primary key autoincrement,
  stable_id text not null default '',
  session_id integer not null references conv_session(id) on delete cascade,
  native_turn_id text,
  turn_index integer not null,
  actor text not null
);

create table if not exists conv_item (
  id integer primary key autoincrement,
  stable_id text not null default '',
  turn_id integer not null references conv_turn(id) on delete cascade,
  item_type text not null,
  tool_name text,
  body_text text,
  payload_json text
);

create table if not exists artifact (
  id integer primary key autoincrement,
  stable_id text not null default '',
  item_id integer not null references conv_item(id) on delete cascade,
  uri text,
  mime text,
  sha256 blob,
  bytes integer
);

create table if not exists evidence_anchor (
  id integer primary key autoincrement,
  stable_id text not null default '',
  item_id integer not null references conv_item(id) on delete cascade,
  selector_type text not null,
  selector_json text not null,
  quoted_text text
);

create table if not exists execution_run (
  id integer primary key autoincrement,
  stable_id text not null default '',
  run_id text not null,
  workspace_id text,
  producer text not null,
  mission_id text,
  flow_id text,
  mode text,
  status text not null,
  started_at_ms integer not null,
  updated_at_ms integer not null,
  last_event_type text not null
);

create table if not exists execution_task (
  id integer primary key autoincrement,
  stable_id text not null default '',
  run_id text not null,
  task_id text not null,
  workspace_id text,
  producer text not null,
  title text,
  status text not null,
  owner_role text,
  updated_at_ms integer not null
);

create table if not exists execution_check (
  id integer primary key autoincrement,
  stable_id text not null default '',
  run_id text not null,
  task_id text,
  name text not null,
  status text not null,
  details text,
  updated_at_ms integer not null
);

create table if not exists execution_approval (
  id integer primary key autoincrement,
  stable_id text not null default '',
  run_id text not null,
  task_id text,
  approval_id text not null,
  kind text,
  status text not null,
  resume_token text,
  updated_at_ms integer not null
);

create table if not exists execution_event (
  id integer primary key autoincrement,
  stable_id text not null default '',
  raw_event_id text not null,
  run_id text not null,
  task_id text,
  producer text not null,
  role text,
  event_type text not null,
  status text,
  body_text text,
  occurred_at_ms integer not null
);

create table if not exists document_record (
  id integer primary key autoincrement,
  stable_id text not null default '',
  document_id text not null,
  workspace_id text,
  producer text not null,
  kind text not null,
  path text,
  title text,
  body_text text,
  artifact_uri text,
  artifact_mime text,
  artifact_sha256 blob,
  artifact_bytes integer,
  updated_at_ms integer not null,
  raw_event_id text not null
);

create table if not exists episode (
  id integer primary key autoincrement,
  stable_id text not null default '',
  workspace_id integer references workspace(id) on delete set null,
  problem_signature text not null,
  status text not null,
  opened_at_ms integer not null,
  closed_at_ms integer
);

create table if not exists episode_member (
  episode_id integer not null references episode(id) on delete cascade,
  turn_id integer not null references conv_turn(id) on delete cascade,
  primary key (episode_id, turn_id)
);

create table if not exists insight (
  id integer primary key autoincrement,
  stable_id text not null default '',
  episode_id integer not null references episode(id) on delete cascade,
  kind text not null,
  summary text not null,
  normalized_text text not null,
  extractor_version text not null,
  confidence real not null,
  stale integer not null default 0
);

create virtual table if not exists insight_fts using fts5(
  summary,
  content='insight',
  content_rowid='id'
);

create table if not exists verification (
  id integer primary key autoincrement,
  stable_id text not null default '',
  episode_id integer not null references episode(id) on delete cascade,
  kind text not null,
  status text not null,
  summary text,
  evidence_id integer references evidence_anchor(id) on delete set null
);

create table if not exists search_doc_redacted (
  id integer primary key autoincrement,
  stable_id text not null default '',
  episode_id integer not null references episode(id) on delete cascade,
  body text not null
);

create virtual table if not exists search_doc_redacted_fts using fts5(
  body,
  content='search_doc_redacted',
  content_rowid='id'
);
