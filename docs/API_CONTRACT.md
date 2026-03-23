# API Contract

이 문서는 현재 릴리스가 실제로 제공하는 public contract만 설명한다.

## Repository Boundary
- 이 저장소는 knowledge kernel, SQLite store, CLI, HTTP API, MCP server를 소유한다.
- canonical write contract는 raw-event sink 하나다.
- canonical read model은 `ingress_receipts -> sessions/entries/artifacts/anchors -> episodes/insights/verifications/procedures`다.
- capture, spool, retry, approval, edge runtime 정본은 외부 시스템이 소유한다.
- compatibility read surface는 한 릴리스 동안만 유지되는 adapter이며 정본 모델이 아니다.

## Runtime Files
- canonical store: `<root>/context.db`
- auth grants: `<root>/auth.json`

## Core Contract
- 모든 결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- dry-run은 apply를 호출하지 않고 plan payload만 반환한다.
- stable id/hash는 canonicalized input만으로 결정론적으로 계산한다.
- sink writer는 raw ledger만 append하고, projection/derivation은 rebuild 가능해야 한다.
- sink write route는 loopback source address만 허용한다.
- 외부 writer 호환을 위해 `source|connector`, `native_session_id`, `native_event_id`, `event_type`, `ts_ms` alias를 유지한다.
- final-form 호환을 위해 root `source.{source_kind,connector_name}` + `events[]` envelope도 수용한다.

## CLI Surface
- `axiomsync init`
- `axiomsync sink plan-append-raw-events`
- `axiomsync sink apply-ingest-plan`
- `axiomsync sink plan-upsert-source-cursor`
- `axiomsync sink apply-source-cursor-plan`
- `axiomsync project rebuild`
- `axiomsync project doctor`
- `axiomsync project plan-auth-grant`
- `axiomsync project plan-admin-grant`
- `axiomsync project apply-auth-grant-plan`
- `axiomsync project apply-admin-grant-plan`
- `axiomsync query search-entries`
- `axiomsync query search-episodes`
- `axiomsync query search-docs`
- `axiomsync query search-insights`
- `axiomsync query search-claims`
- `axiomsync query search-procedures`
- `axiomsync query find-fix`
- `axiomsync query find-decision`
- `axiomsync query find-runbook`
- `axiomsync query get-evidence-bundle`
- `axiomsync query get-session`
- `axiomsync query get-entry`
- `axiomsync query get-artifact`
- `axiomsync query get-anchor`
- `axiomsync query get-episode`
- `axiomsync query get-claim`
- `axiomsync query get-procedure`
- `axiomsync compat get-case`
- `axiomsync compat get-thread`
- `axiomsync compat get-runbook`
- `axiomsync compat get-task`
- `axiomsync mcp serve`
- `axiomsync serve`

## HTTP Surface
### Canonical write
- `GET /health`
- `POST /sink/raw-events/plan`
- `POST /sink/raw-events/apply`
- `POST /sink/source-cursors/plan`
- `POST /sink/source-cursors/apply`

### Admin rebuild
- `POST /admin/rebuild/projection`
- `POST /admin/rebuild/derivations`
- `POST /admin/rebuild/index`

### Canonical read
- `GET /api/sessions/{id}`
- `GET /api/entries/{id}`
- `GET /api/artifacts/{id}`
- `GET /api/anchors/{id}`
- `GET /api/episodes/{id}`
- `GET /api/claims/{id}`
- `GET /api/procedures/{id}`
- `POST /api/query/search-entries`
- `POST /api/query/search-episodes`
- `POST /api/query/search-docs`
- `POST /api/query/search-insights`
- `POST /api/query/search-claims`
- `POST /api/query/search-procedures`
- `POST /api/query/find-fix`
- `POST /api/query/find-decision`
- `POST /api/query/find-runbook`
- `POST /api/query/evidence-bundle`

### Compatibility read
- `GET /api/cases/{id}`
- `GET /api/threads/{id}`
- `GET /api/runbooks/{id}`
- `GET /api/runs`
- `GET /api/runs/{id}`
- `GET /api/tasks/{id}`
- `GET /api/documents/{id}`
- `GET /api/evidence/{id}`
- `POST /mcp`

canonical noun은 `sessions`, `entries`, `artifacts`, `anchors`, `episodes`, `insights`, `procedures`다.
compatibility noun은 `claims`, `cases`, `threads`, `runbooks`, `runs`, `tasks`, `documents`, `evidence`다.
`task` compatibility id는 독립 정본이 아니라 `session_kind == "task"`인 session id를 그대로 사용한다.

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
  - `session://{id}`
  - `episode://{id}`
  - `insight://{id}`
  - `procedure://{id}`
  - `axiom://sessions/{id}`
  - `axiom://entries/{id}`
  - `axiom://artifacts/{id}`
  - `axiom://anchors/{id}`
  - `axiom://episodes/{id}`
  - `axiom://claims/{id}`
  - `axiom://insights/{id}`
  - `axiom://procedures/{id}`
- compatibility resources:
  - `axiom://cases/{id}`
  - `axiom://threads/{id}`
  - `axiom://runbooks/{id}`
  - `axiom://tasks/{id}`
- canonical tools:
  - `search_entries`
  - `search_episodes`
  - `search_docs`
  - `search_insights`
  - `search_claims`
  - `search_procedures`
  - `find_fix`
  - `find_decision`
  - `find_runbook`
  - `get_evidence_bundle`
  - `get_session`
  - `get_entry`
  - `get_artifact`
  - `get_anchor`
- compatibility tools:
  - `get_case`
  - `get_thread`
  - `get_runbook`
  - `get_task`

## Sink Contract
- raw append는 `AppendRawEventsRequest -> IngestPlan -> apply_ingest` 순서만 허용한다.
- source cursor upsert는 `UpsertSourceCursorRequest -> SourceCursorUpsertPlan -> apply_source_cursor_upsert` 순서만 허용한다.
- `apply-*`는 original request가 아니라 validated plan payload만 받는다.
- `source_cursor`는 kernel 내부 operator metadata이며, spool/retry/approval/run-state 정본은 아니다.
- health 응답은 `pending_projection_count`, `pending_derived_count`, `pending_index_count`를 포함한다.
- docs-package fixture 회귀는 `kernel_sink_contract.json` schema validation까지 포함한다.

## Auth And Scope
- workspace-scoped HTTP read surface는 workspace bearer token을 요구한다.
- admin rebuild surface와 admin MCP call은 global admin bearer token을 요구한다.
- MCP HTTP binding은 resource/tool별 workspace requirement를 강제한다.
- sink write surface는 bearer token 없이 loopback source address만 허용한다.
- `auth.json`에는 hashed workspace grants와 hashed admin token만 저장한다.
