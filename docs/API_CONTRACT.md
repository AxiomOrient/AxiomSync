# API Contract

이 문서는 저장소가 보장하는 안정 계약만 적습니다.

## Repository Boundary
- This repository owns the runtime library and operator CLI only.
- Web viewer/server and mobile FFI are companion projects outside this repository.

## URI Contract
- Canonical URI: `axiom://{scope}/{path}`
- Core scopes: `resources`, `user`, `agent`, `session`, `events`
- Internal scopes: `temp`, `queue`
- `queue` scope는 시스템 작업 외 쓰기 금지
- `events` scope는 시간 정렬 이벤트 로그 전용이다

## Persistence Contract
- Canonical local store: `<root>/context.db`
- `context.db`는 큐, 체크포인트, OM 상태, 검색 영속 상태, v3 도메인 객체(resources, events, links)를 함께 저장한다.
- 런타임 검색은 메모리 인덱스로 수행하되, 부팅 시 persisted search state에서 복원한다.
- 런타임은 legacy DB 파일명을 탐색하거나 자동 마이그레이션하지 않는다.
- Persistence backend는 SQLite로 고정한다.
- Schema version은 `system_kv.context_schema_version = 'v3'`으로 추적한다.
- FTS projection version은 `system_kv.search_docs_fts_schema_version` 으로 추적한다.
- release contract version은 `system_kv.release_contract_version` 으로 추적한다.
- migration / repair 실행 이력은 `schema_migration_runs`, `repair_runs` 테이블에 기록된다.

## Retrieval Contract
- Public query surface:
  - `find(query, target_uri?, limit?, score_threshold?, filter?)`
  - `search(query, target_uri?, session?, limit?, score_threshold?, filter?)`
  - `search_with_request(SearchRequest { ..., runtime_hints })`
- Runtime retrieval backend policy는 `memory_only`다. In-memory index가 runtime ranking을 담당한다.
- SQLite `search_docs` / `search_docs_fts`는 persisted projection이자 FTS acceleration layer다. namespace/kind/event_time 필터가 있는 쿼리는 `search_documents_fts_filtered`를 사용한다.
- 기본 검색 스코프는 `resources`다. `events` 스코프는 `target_uri` 명시 또는 쿼리 필터를 통해 접근해야 한다.
- `search_docs_fts` bootstrap completeness 는 `system_kv.search_docs_fts_schema_version` marker 로 추적할 수 있고, marker 가 없으면 rebuild 가 재시도된다.
- `FindResult.query_results` 와 `hit_buckets` 가 canonical retrieval result shape 다.
- 기본 JSON surface 는 canonical only 다.
- `--compat-json` 사용 시에만 `memories`, `resources`, `skills` 호환 배열이 직렬화된다.
- `FindResult.memories()`, `resources()`, `skills()` 와 직렬화 호환 배열은 canonical source 가 아니라 `query_results + hit_buckets` 에서 파생된다.
- trace JSON 은 `scope_decision`, `filter_routing_reason`, `restore_source`, `fts_fallback_used` 필드를 포함한다.

## Filesystem And Resource Contract
- `initialize()`
- `add_resource(path_or_url, target?, reason?, instruction?, wait, wait_mode?, timeout?)`
- `wait_processed(timeout?)`
- `ls(uri, recursive, simple)`
- `read(uri)`
- `mkdir(uri)`
- `rm(uri, recursive)`
- `mv(from_uri, to_uri)`

## v3 Event, Link, Repo Contract
- `add_event(AddEventRequest) -> EventRecord`
- `add_events(Vec<AddEventRequest>) -> Vec<EventRecord>`
  - `attrs` 크기가 4KB 초과이거나 `raw_payload` 필드를 포함하면 out-of-line 저장 후 `externalized` 참조로 교체된다.
- `link_records(LinkRequest) -> LinkRecord`
  - `relation` 필드는 ascii 알파뉴메릭과 `-`, `_`만 허용하며 소문자로 정규화된다.
- `mount_repo(RepoMountRequest) -> RepoMountReport`
  - 소스 디렉터리를 resources 스코프에 복사하고, ResourceRecord를 생성한다.
- `plan_event_archive(archive_id, EventQuery, archive_reason?, archived_by?) -> EventArchivePlan`
- `execute_event_archive(EventArchivePlan) -> EventArchiveReport`
  - destructive archive 는 plan -> execute 2단계로만 노출된다.
  - `RetentionClass::Ephemeral`인 경우 execute 시점에 이벤트는 search_docs에서 제거되고 attrs_json이 archive 참조로 교체된다.
  - 조회 결과가 비어 있거나 retention class가 혼재하면 실패한다.

## Operator Command Contract
- `axiomsync doctor storage --json`
- `axiomsync doctor retrieval --json`
- `axiomsync migrate inspect --json`
- `axiomsync migrate apply --backup-dir <dir> --json`
- `axiomsync release verify --json`

## Session And Memory Contract
- `session(session_id?)`
- `sessions()`
- `delete(session_id)`
- `promote_session_memories(request)`
- `checkpoint_session_archive_only(session_id)`

## OM Boundary Contract
- Pure OM contract and transform 계층은 vendored engine 아래에 유지한다.
- Runtime and persistence policy 계층은 `axiomsync`가 담당한다.
- Prompt and response header strict fields:
  - `contract_name`
  - `contract_version`
  - `protocol_version`
- XML/JSON fallback content도 contract marker 검증을 통과해야 수용된다.
- Search hint는 OM snapshot read-model 기준으로 구성한다.

## Release Gate Contract
- Repository-grade checks:
  - `bash scripts/quality_gates.sh`
  - `bash scripts/release_pack_strict_gate.sh --workspace-dir <repo>`
- Contract integrity gate는 다음을 검증한다:
  - contract execution probe
  - episodic API probe
  - prompt signature version-bump policy
  - ontology contract probe
- `HEAD~1` 미존재, shallow history, path rename/cutover 등으로 이전 정책 소스를 읽을 수 없을 때는 current workspace policy shape 검증으로 fallback 한다.

## Dependency Contract
- `axiomsync` must not declare an `episodic` crate dependency.
- Required vendored contract file: `crates/axiomsync/src/om/engine/prompt/contract.rs`
- Required vendored engine entry: `crates/axiomsync/src/om/engine/mod.rs`
- `Cargo.lock` must not resolve an `episodic` package for `axiomsync`.

## Non-goals
- Web viewer implementation detail
- Mobile FFI surface design
- Experimental benchmark internals
- Historical rollout logs

## References
- [Runtime Architecture](./RUNTIME_ARCHITECTURE.md)
- [Retrieval Architecture](./RETRIEVAL_ARCHITECTURE.md)
- [Release Runbook](./RELEASE_RUNBOOK.md)
