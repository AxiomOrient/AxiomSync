# Retrieval Architecture

이 문서는 검색 경로의 source of truth 를 짧게 고정합니다.

## Canonical Shape
- query entrypoint: `find`, `search`, `search_with_request`
- runtime retrieval backend policy: `memory_only`
- runtime ranking and top-k: in-memory index
- persisted source of truth: SQLite projection (`search_docs`)

## SQLite Role
- `context.db` 는 queue, checkpoint, OM state, persisted search projection 을 함께 저장한다.
- `search_docs` 는 검색 문서의 canonical persisted projection 이다. 파일 인덱싱, 리소스, 이벤트 모두 이 테이블을 통해 search_docs에 투영된다.
- `search_docs` 에는 v3 메타데이터 컬럼(`namespace`, `kind`, `event_time`, `source_weight`, `freshness_bucket`)이 포함된다.
- `search_doc_tags` 는 tag projection 이다.
- `search_docs_fts` 는 FTS5 lexical acceleration layer 다.
- `search_documents_fts_filtered(query, namespace?, kind?, min_event_time?, limit)` 로 namespace/kind/event_time 조건부 쿼리를 SQLite에서 직접 수행할 수 있다. 이는 v3 이벤트와 리소스 검색에 사용된다.

## Search Projection Paths
세 가지 경로가 `search_docs`를 갱신한다:

| 경로 | 호출 지점 | namespace/kind 컬럼 |
|---|---|---|
| 파일 인덱싱 | `index_file_entry` → `persist_search_document` | 태그에서 추출 (예: `kind:adr`) |
| v3 리소스 | `persist_resource_search_document` | `ResourceRecord.namespace`/`kind` 직접 사용 |
| v3 이벤트 | `persist_event_search_document` | `EventRecord.namespace`/`kind` 직접 사용 |

## FTS Bootstrap Safety
- FTS bootstrap completeness 는 `system_kv.search_docs_fts_schema_version` marker 로 추적한다.
- marker 가 없거나 schema version 이 다르면 migration 이 `search_docs_fts` rebuild 를 다시 수행한다.
- marker 는 rebuild 성공 후에만 갱신된다.
- 따라서 virtual table 이 이미 있어도 interrupted migration 뒤에는 안전하게 backfill 이 재시도된다.

## Query Path
### 파일 기반 문서 (resources/user/agent/session 스코프)
1. ingest/replay 가 `search_docs` projection 을 갱신한다.
2. FTS5 trigger 가 `search_docs_fts` 를 동기화한다.
3. startup 에서 memory index 를 복원한다.
4. query 시 runtime 이 memory index 에서 선택한다.

### v3 이벤트 (`events` 스코프)
1. `add_events` → `events` 테이블 → `persist_event_search_document` → `search_docs` 갱신.
2. FTS5 trigger 가 `search_docs_fts` 를 동기화한다.
3. `sync_events_runtime_index` 가 `search_docs`에서 읽어 memory index 를 즉시 갱신한다.
4. `target_uri` 가 없더라도 event-like `SearchFilter` 가 있으면 planner 가 primary scope 를 `events` 로 바꾼다.

### v3 리소스 (`mount_repo`)
1. `persist_resource_search_document` → `search_docs` 갱신.
2. `sync_runtime_index` 가 `search_docs`에서 읽어 memory index 를 즉시 갱신한다.
3. 파일 인덱싱 파이프라인도 같은 URI 하위 파일들을 별도로 인덱싱한다.

## Scope and Filter Routing
- primary scope 우선순위는 `target_uri` → `SearchFilter` → query intent → default 다.
- `SearchOptions.target_uri` 를 `axiom://events/...` 로 설정하면 events 스코프를 검색한다.
- `SearchFilter.start_time` 또는 `SearchFilter.end_time` 이 있으면 planner 는 `events` 를 primary scope 로 잡는다.
- `SearchFilter.kind=incident|run|deploy|log|trace` 이면 planner 는 `events` 를 primary scope 로 잡는다.
- `SearchFilter.kind=contract|adr|runbook|repository` 이면 planner 는 `resources` 를 primary scope 로 잡는다.
- `SearchFilter.kind` / `SearchFilter.namespace_prefix` / `SearchFilter.start_time` / `SearchFilter.end_time` 필터는 in-memory index의 tag 기반 필터링과 SQLite FTS filtered query 모두에서 적용된다.
- 상세 규칙과 최근 회귀 분석은 이 문서의 `Planner Rules` 섹션을 따른다.

## Compatibility Surface
- `FindResult.query_results` 가 canonical ordered hit list 다.
- `FindResult.hit_buckets` 가 hit category 의 canonical index map 이다.
- 기본 JSON 응답은 canonical only 다.
- `--compat-json` 사용 시에만 `memories`, `resources`, `skills` 호환 배열이 직렬화된다.
- `FindResult.memories()`, `resources()`, `skills()` 는 derived iterator view 다.
- derived view 와 JSON compatibility 배열은 독립 source of truth 가 아니라 `query_results + hit_buckets` 에서 파생된다.

## Planner Rules
planner 는 아래 순서로 primary scope 를 정한다.

1. `target_uri`
2. `SearchFilter`
3. query intent
4. default fallback

현재 filter routing 규칙은 아래와 같다.

- `start_time` 또는 `end_time` 이 있으면 `events`
- `kind=incident|run|deploy|log|trace` 이면 `events`
- `kind=contract|adr|runbook|repository` 이면 `resources`
- 그 외 kind 는 planner 가 scope 를 강제하지 않고 intent fallback 으로 내린다

제약:

- `namespace_prefix` 만으로는 아직 scope 를 강제하지 않는다.
- event/resource 혼합 의도 질의는 아직 weighted multi-scope planning 으로 풀지 않고 단일 primary scope 를 사용한다.

## Legacy Boundary
- 런타임은 legacy DB 파일명 탐색이나 별도 저장소 cutover 를 지원하지 않는다.
- 현재 릴리스 라인은 startup 시 필요한 현재 스키마만 생성한다.
- 구버전 schema 호환, 별도 migration, legacy repair path 는 지원하지 않는다.

## Evidence
- FTS projection sync: `cargo test -q -p axiomsync state::tests::search_documents_fts_tracks_upsert_and_remove`
- interrupted bootstrap recovery: `cargo test -q -p axiomsync state::tests::migration_rebuilds_fts_when_bootstrap_marker_is_missing`
- runtime lexical comparison: `cargo test -q -p axiomsync client::tests::core_editor_retrieval::fts5_prototype_matches_runtime_top_hit_for_exact_lexical_query`
- v3 resource projection: `cargo test -q -p axiomsync state::search::tests::resource_projection_persists_v3_metadata_columns`
- v3 event filtered FTS: `cargo test -q -p axiomsync state::search::tests::event_projection_supports_filtered_fts_queries`
- planner filter routing: `cargo test -q -p axiomsync retrieval::planner::tests::event_filter_without_target_switches_primary_scope_to_events`
- canonical/compat contract: `cargo test -q -p axiomsync models::search::tests::find_result_serialization_defaults_to_canonical_contract`
- compat presenter: `cargo test -q -p axiomsync models::search::tests::find_result_compat_view_includes_legacy_bucket_arrays`
- repo file namespace inheritance: `cargo test -q -p axiomsync client::indexing::tests::index_file_entry_inherits_namespace_tag_from_nearest_resource`
