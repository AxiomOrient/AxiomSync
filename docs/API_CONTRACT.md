# API Contract

이 문서는 현재 릴리스 라인이 실제로 보장하는 public surface만 적습니다.

## Repository Boundary
- 이 저장소는 AxiomSync kernel, CLI, HTTP API, MCP server, local web UI를 소유한다.
- canonical write contract는 raw-only `sink` surface다.
- canonical read model은 `record -> view -> knowledge` 3층이다.
- capture extension, daemon, fallback worker는 이 저장소 범위 밖이다.
- 과거 v3 resource/event/link runtime, migration, release-pack surface는 현재 릴리스 범위에 포함되지 않는다.

## Runtime Files
- canonical store: `<root>/context.db`
- auth grants: `<root>/auth.json`

## Core Contract
- 모든 결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- `dry-run`은 apply를 실행하지 않고 plan payload만 반환한다.
- stable id/hash/path는 canonicalized input에서 결정론적으로 계산한다.
- raw transcript 전체를 직접 FTS하지 않고 `search_doc_redacted`와 evidence fallback을 사용한다.
- raw ledger는 agent-semantic record를 보존하고, projection은 query-worthy state만 materialize한다.

## CLI Surface
- `axiomsync init`
- `axiomsync sink plan-append-raw-events`
- `axiomsync sink apply-ingest-plan`
- `axiomsync sink plan-upsert-source-cursor`
- `axiomsync sink apply-source-cursor-plan`
- `axiomsync project plan-rebuild`
- `axiomsync project apply-replay-plan`
- `axiomsync project plan-purge`
- `axiomsync project apply-purge-plan`
- `axiomsync project doctor`
- `axiomsync project plan-auth-grant`
- `axiomsync project plan-admin-grant`
- `axiomsync project apply-auth-grant-plan`
- `axiomsync project apply-admin-grant-plan`
- `axiomsync derive plan`
- `axiomsync derive apply-plan`
- `axiomsync search`
- `axiomsync runbook`
- `axiomsync mcp serve`
- `axiomsync web`

`sink *`는 canonical kernel write surface다.
`runbook`은 legacy compatibility surface다.

## HTTP Surface
### Main router
- `GET /health`
- `GET /`
- `GET /cases/{id}`
- `GET /episodes/{id}`
- `POST /sink/raw-events/plan`
- `POST /sink/raw-events/apply`
- `POST /sink/source-cursors/plan`
- `POST /sink/source-cursors/apply`
- `POST /project/rebuild/plan`
- `POST /project/rebuild/apply`
- `POST /project/purge/plan`
- `POST /project/purge/apply`
- `POST /derive/plan`
- `POST /derive/apply`
- `GET /api/cases`
- `GET /api/cases/{id}`
- `GET /api/episodes`
- `GET /api/runbooks/{id}`
- `GET /api/threads/{id}`
- `GET /api/runs`
- `GET /api/runs/{id}`
- `GET /api/tasks/{id}`
- `GET /api/documents`
- `GET /api/documents/{id}`
- `GET /api/evidence/{id}`
- `POST /mcp`

canonical read nouns는 `cases`, `threads`, `runs`, `tasks`, `documents`, `evidence`다.
`episodes`와 `runbooks`는 compatibility route로 유지된다.

## MCP Surface
- transports: `stdio`, HTTP
- methods:
  - `initialize`
  - `roots/list`
  - `resources/list`
  - `resources/read`
  - `tools/list`
  - `tools/call`
- canonical resources:
  - `axiom://cases/{id}`
  - `axiom://threads/{id}`
  - `axiom://runs/{id}`
  - `axiom://documents/{id}`
  - `axiom://evidence/{id}`
- compatibility resources:
  - `axiom://episode/{id}`
  - `axiom://thread/{id}`
- canonical tools:
  - `search_cases`
  - `get_case`
  - `get_thread`
  - `get_evidence`
  - `search_commands`
  - `list_runs`
  - `get_run`
  - `get_task`
  - `list_documents`
  - `get_document`
- compatibility tools:
  - `search_episodes`
  - `get_runbook`

## Sink Contract
- `plan-*` route/command는 mutation 없이 plan payload만 반환한다.
- `apply-*` route/command는 original request가 아니라 validated plan payload만 입력으로 받는다.
- raw append는 `AppendRawEventsRequest -> ConnectorBatchInput -> IngestPlan -> apply_ingest` 순서만 허용한다.
- source cursor upsert는 `UpsertSourceCursorRequest -> SourceCursorUpsertPlan -> apply_source_cursor_upsert` 순서만 허용한다.
- canonical universal envelope는 `native_schema_version = "agent-record-v1"`를 허용한다.

## Auth And Scope
- workspace-scoped HTTP read surface requires workspace bearer auth
- admin HTTP and web surface requires global admin bearer auth
- MCP HTTP binding enforces workspace scope
- sink write routes는 bearer auth를 요구하지 않지만 loopback source address만 허용한다
- `auth.json` stores hashed workspace grants and hashed global admin tokens, and is written with owner-only permissions on Unix
