# API Contract

## 0. Scope
이 문서는 현재 AxiomMe 런타임의 핵심 계약만 정의합니다.
상세 구현/실험 옵션은 제외합니다.

## 1. URI Contract
- Canonical URI: `axiom://{scope}/{path}`
- Core scopes: `resources`, `user`, `agent`, `session`
- Internal scopes: `temp`, `queue`
- Rule: `queue` scope는 시스템 작업 외 쓰기 금지

## 2. Core Client Surface

### Filesystem/Resource
- `initialize()`
- `add_resource(path_or_url, target?, reason?, instruction?, wait, wait_mode?, timeout?)`
- `wait_processed(timeout?)`
- `ls(uri, recursive, simple)`
- `read(uri)`
- `mkdir(uri)`
- `rm(uri, recursive)`
- `mv(from_uri, to_uri)`

### Retrieval
- `find(query, target_uri?, limit?, score_threshold?, filter?)`
- `search(query, target_uri?, session?, limit?, score_threshold?, filter?)`
- `search_with_request(SearchRequest { ..., runtime_hints })`

### Session/Memory
- `session(session_id?)`
- `sessions()`
- `delete(session_id)`
- `promote_session_memories(request)`
- `checkpoint_session_archive_only(session_id)`

## 3. OM v2 Boundary Contract
- Pure OM contract/transform 계층: `episodic`
- Runtime/persistence 계층: `axiomme-core`
- Prompt/response header strict fields:
  - `contract_name`
  - `contract_version`
  - `protocol_version`
- Fallback content(XML/JSON)도 contract marker 검증을 통과해야 수용
- Search hint는 OM snapshot(read-model) 기준으로 구성

## 4. Release Gate Contract
- Contract integrity gate는 다음을 검증:
  - contract execution probe
  - episodic API probe
  - prompt signature version-bump policy
  - ontology contract probe
- `HEAD~1` 미존재 환경(shallow/squash)에서는 current policy shape 검증으로 fallback

## 5. Dependency Contract (episodic)
- Required source: `https://github.com/AxiomOrient/episodic.git`
- Required revision: `53dfe97bc7df8e32dbee5f7b2be862a6da9171c5`
- Required compatible line: `0.2.x`

## 6. Non-goals
- 웹 뷰어 구현 상세
- 실험/벤치마크 내부 리포트 포맷
- 과거 릴리즈 이행 기록

## 7. Canonical References
- [Architecture](./ARCHITECTURE.md)
- [Ontology Evolution Policy](./ONTOLOGY_SCHEMA_EVOLUTION_POLICY.md)
