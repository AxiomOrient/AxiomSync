# API Contract

이 문서는 현재 릴리스가 실제로 제공하는 public contract만 설명한다.

## Repository Boundary
- 이 저장소는 knowledge kernel, SQLite store, CLI, HTTP API, MCP server를 소유한다.
- canonical write contract는 raw-event sink 하나다.
- canonical read model은 `case / thread / run / task / document / evidence`다.
- 내부 projection/derivation은 `ingress_receipts -> sessions/entries/artifacts/anchors -> episodes/insights/verifications/procedures` 순서로 유지한다.
- capture, spool, retry, approval, edge runtime 정본은 외부 시스템이 소유한다.

## Runtime Files
- canonical store: `<root>/context.db`
- auth grants: `<root>/auth.json`

## Core Contract
- 모든 결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- dry-run은 apply를 호출하지 않고 plan payload만 반환한다.
- stable id/hash는 canonicalized input만으로 결정론적으로 계산한다.
- sink writer는 raw ledger만 append하고, projection/derivation은 rebuild 가능해야 한다.
- sink write route는 loopback source address만 허용한다.
- canonical append request는 `AppendRawEventsRequest { batch_id, producer, received_at_ms, events[] }`다.
- canonical cursor request는 `UpsertSourceCursorRequest { connector, cursor_key, cursor_value, updated_at_ms }`다.
- relay same-host adapter semantics는 [`RELAY_INTEROP.md`](./RELAY_INTEROP.md) 에서 고정한다.
- canonical wire example은 `native_event_id` 를 사용하지만, 현재 구현은 compatibility input으로 `native_entry_id`, optional `artifacts`, `hints` field도 수용한다.
- `RawEvent.event_type`는 고정 taxonomy만 허용한다:
  - `message_captured`
  - `selection_captured`
  - `command_started`
  - `command_finished`
  - `artifact_emitted`
  - `verification_recorded`
  - `task_state_imported`
  - `approval_requested`
  - `approval_resolved`
  - `note_recorded`

## CLI Surface
- `axiomsync-cli init`
- `axiomsync-cli sink plan-append-raw-events`
- `axiomsync-cli sink apply-ingest-plan`
- `axiomsync-cli sink plan-upsert-source-cursor`
- `axiomsync-cli sink apply-source-cursor-plan`
- `axiomsync-cli project plan-projection`
- `axiomsync-cli project apply-projection-plan`
- `axiomsync-cli project plan-derivations`
- `axiomsync-cli project apply-derivation-plan`
- `axiomsync-cli project plan-rebuild`
- `axiomsync-cli project apply-replay-plan`
- `axiomsync-cli project doctor`
- `axiomsync-cli project plan-auth-grant`
- `axiomsync-cli project plan-admin-grant`
- `axiomsync-cli project apply-auth-grant-plan`
- `axiomsync-cli project apply-admin-grant-plan`
- `axiomsync-cli sink import-cli-run`
- `axiomsync-cli sink import-work-state`
- `axiomsync-cli query search-cases`
- `axiomsync-cli query get-case`
- `axiomsync-cli query get-thread`
- `axiomsync-cli query get-run`
- `axiomsync-cli query get-task`
- `axiomsync-cli query get-document`
- `axiomsync-cli query get-evidence`
- `axiomsync-cli mcp serve`
- `axiomsync-cli serve`

## HTTP Surface
### Canonical write
- `GET /health`
- `POST /sink/raw-events/plan`
- `POST /sink/raw-events/apply`
- `POST /sink/source-cursors/plan`
- `POST /sink/source-cursors/apply`

### Admin rebuild
- `POST /admin/projection/plan`
- `POST /admin/projection/apply`
- `POST /admin/derivations/plan`
- `POST /admin/derivations/apply`
- `POST /admin/replay/plan`
- `POST /admin/replay/apply`

### Canonical read
- `GET /api/cases/{id}`
- `GET /api/threads/{id}`
- `GET /api/runs`
- `GET /api/runs/{id}`
- `GET /api/tasks/{id}`
- `GET /api/documents/{id}`
- `GET /api/evidence/{id}`
- `POST /api/query/search-cases`
- `POST /mcp`

public canonical noun은 `case`, `thread`, `run`, `task`, `document`, `evidence`다.
`session`, `entry`, `artifact`, `anchor`, `episode`, `insight`, `procedure`는 내부 projection/derivation 모델이다.

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
  - `axiom://tasks/{id}`
  - `axiom://documents/{id}`
  - `axiom://evidence/{id}`
- canonical tools:
  - `search_cases`
  - `get_case`
  - `get_thread`
  - `get_run`
  - `get_task`
  - `get_document`
  - `get_evidence`
  - `list_runs`
  - `list_documents`

## Sink Contract
- raw append는 `AppendRawEventsRequest -> IngestPlan -> apply_ingest` 순서만 허용한다.
- source cursor upsert는 `UpsertSourceCursorRequest -> SourceCursorUpsertPlan -> apply_source_cursor_upsert` 순서만 허용한다.
- projection rebuild는 `build_projection_plan -> apply_projection_plan` 순서를 지원한다.
- derivation rebuild는 `build_derivation_plan -> apply_derivation_plan` 순서를 지원한다.
- full replay rebuild는 `build_replay_plan -> apply_replay` 순서를 지원한다.
- `apply-*`는 original request가 아니라 validated plan payload만 받는다.
- duplicate append는 idempotent success로 처리하고 `skipped_dedupe_keys`로 노출한다.
- duplicate source cursor upsert도 idempotent success semantics를 따른다.
- `source_cursor`는 kernel 내부 operator metadata이며, spool/retry/approval/run-state 정본은 아니다.
- health 응답은 `pending_projection_count`, `pending_derived_count`, `pending_index_count`를 포함한다.
- fixture 회귀는 [`contracts/kernel_sink_contract.json`](./contracts/kernel_sink_contract.json) schema validation까지 포함한다.
- relay interop fixture 회귀는 same-host loopback HTTP sink sequence까지 포함한다.

## Auth And Scope
- workspace-scoped HTTP read surface는 workspace bearer token을 요구한다.
- admin rebuild surface와 admin MCP call은 global admin bearer token을 요구한다.
- MCP HTTP binding은 resource/tool별 workspace requirement를 강제한다.
- sink write surface는 bearer token 없이 loopback source address만 허용한다.
- `auth.json`에는 hashed workspace grants와 hashed admin token만 저장한다.
