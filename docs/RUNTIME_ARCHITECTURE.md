# Runtime Architecture

핵심 구조: `axiom://` URI, 단일 `context.db`, `memory_only` 검색 런타임, 명시적인 세션/OM 상태, v3 도메인 객체(Resources, Events, Links).

## Repository Boundary
- Inside this repository: runtime library, operator CLI, release scripts
- Outside this repository: web companion, mobile FFI companion, app-specific frontend shells

## Layers
- Interface: CLI parses commands and delegates to runtime
- Facade: `AxiomSync` coordinates filesystem, state, retrieval, session, release; thin orchestration API (`facade.rs`)
- Storage: `LocalContextFs` + `SqliteStateStore`
- Retrieval: `search_docs` + `search_docs_fts` persisted state, `memory_only` runtime query path
- Session and OM: explicit session state + vendored OM engine under `src/om/engine`
- Release and Evidence: benchmark, eval, security, operability, contract gates

## URI Scopes
- `resources`: 파일 시스템 기반 컨텍스트 문서
- `user`, `agent`: 메모리, 스킬
- `session`: 세션 메시지
- `events`: 시간 정렬 이벤트 로그 (v3)
- `temp`, `queue`: 시스템 내부 전용 (쓰기 금지)

## Storage Schema (v3)
`context.db`는 다음 테이블 그룹을 함께 저장한다:
- **검색 영속 상태**: `search_docs`, `search_doc_tags`, `search_docs_fts` (FTS5 virtual table)
- **v3 도메인 객체**: `resources`, `events`, `links`
- **세션 상태**: `sessions`
- **큐 및 체크포인트**: `outbox`, `queue_checkpoint`
- **OM 상태**: `om_records`, `om_entries`, `om_observation_chunks` 등
- **시스템 메타**: `system_kv`, `reconcile_runs`, `trace_index`, `memory_promotion_checkpoints`

`context.db` schema version은 `system_kv.context_schema_version = 'v3'`으로 추적한다.

## Main Data Flows
- Bootstrap: filesystem scopes 생성, runtime state restore
- File Ingest: resource ingest → filesystem indexing → search_docs projection update
- v3 Event Ingest: `add_events` → `events` table → `search_docs` projection → memory index sync
- v3 Resource Ingest: `mount_repo` → `resources` table + filesystem copy → search projection
- v3 Link: `link_records` → `links` table
- Query: `memory_only` retrieval (SQLite FTS accelerates namespace/kind/time filters), trace 기록
- Session and OM: session memory update, restart-safe checkpoint/replay
- Release: executable gate 실행

## Retrieval Architecture
- Runtime ranking은 in-memory index가 담당한다.
- Startup 시 `search_docs`에서 memory index를 복원한다.
- FTS5(`search_docs_fts`)는 lexical acceleration layer다. `search_documents_fts_filtered`로 namespace/kind/event_time 조건부 쿼리를 SQLite에서 직접 수행할 수 있다.
- 기본 검색 스코프는 `resources`다. `events` 스코프는 `target_uri` 명시 또는 필터를 통해 접근한다.

## Boundary Rules
- Side effects belong at filesystem and state boundaries, not inside pure selection logic.
- Startup is a hard cutover to `context.db`; legacy DB discovery and migration are out of scope.
- Runtime startup creates and uses only the current v3 schema; compatibility repair paths are out of scope.
- `queue` scope is system-owned for writes.
- Vendored OM code remains explicit under `src/om/engine`; runtime-only policy stays in `axiomsync::om`.

## Known Behavioral Notes
- `mount_repo` 후 전역 reindex가 실행되면, root resource URI의 search_docs 항목에서 `namespace`/`kind` 컬럼이 디렉터리 인덱서에 의해 덮어써질 수 있다. `resources` 테이블의 원본 데이터는 보존된다.
- `plan_event_archive`로 대상 이벤트를 먼저 확정하고 `execute_event_archive`로 ephemeral 이벤트를 압축하면, 이벤트는 search_docs에서 제거되지만 `events` 테이블에는 남아 `query_events`로 조회 가능하다 (attrs_json이 archive 참조로 교체됨).
