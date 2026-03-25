# Runtime Architecture

현재 구조는 local-first universal agent memory kernel이다.

## Workspace Roles
- `axiomsync-domain`: canonical types, validation, deterministic helpers
- `axiomsync-kernel`: pure planning logic, query shaping, application service
- `axiomsync-store-sqlite`: SQLite repository + transaction/apply adapter
- `axiomsync-http`: unified HTTP router and auth enforcement
- `axiomsync-mcp`: MCP request/response adapter
- `axiomsync-cli`: CLI contract and command dispatch
- workspace root: release docs, verification script, and Cargo workspace composition

## Data Flow
1. external input enters through CLI or HTTP sink
2. kernel parses and normalizes input into deterministic rows
3. kernel produces plan objects
4. SQLite adapter applies plans inside transaction boundaries
5. query surfaces read canonical views and evidence-backed knowledge

External collectors or edge runtimes live outside this repository and write only through the sink seam.

## Storage Model
- raw ledger: `ingress_receipts`
- cursor state: `source_cursor`
- projection layer:
  - `sessions`
  - `actors`
  - `entries`
  - `artifacts`
  - `anchors`
- derivation layer:
  - `episodes`
  - `insights`
  - `insight_anchors`
  - `verifications`
  - `claims`
  - `claim_evidence`
  - `procedures`
  - `procedure_evidence`
- retrieval layer:
  - `search_docs`
  - `search_docs_fts`
  - `episode_search_fts`
  - `insight_search_fts`
  - `claim_search_fts`
  - `procedure_search_fts`

public canonical noun은 `case / thread / run / task / document / evidence`다. `session`, `entry`, `artifact`, `anchor`, `episode`, `insight`, `claim`, `procedure`는 내부 projection/derivation 모델로 유지한다.

## Boundary Rules
- pure logic stays in `axiomsync-kernel`
- Parse -> Normalize -> Plan stays side-effect free
- Apply stays behind repository/auth adapters
- dry-run never mutates store state
- MCP adapter, trusted CLI import compiler, and SQLite bootstrap guards now all follow the same explicit parse/normalize/plan/apply boundary
- sink write routes are unauthenticated but loopback-only
- workspace-scoped read routes require workspace bearer auth
- collection reads and search require an explicit workspace selector before workspace auth is evaluated
- admin rebuild, web UI, and MCP admin operations require admin bearer auth
- external edge repositories write through `/sink/*` on the main `serve` router or equivalent CLI plan/apply flow
- capture, spool, retry, approval, browser integration are outside this repository

## Retrieval Model
- projection search materializes bounded case-oriented documents into `search_docs`
- ranking combines exact match, multi-token body match, evidence density, and verification signal
- raw transcript 전체를 직접 인덱싱하지 않고, projection/derivation에서 나온 bounded text만 검색 대상으로 쓴다
- execution/document records는 raw ledger에 남지만 canonical case derivation을 직접 오염시키지 않는다

## Schema Lifecycle
- `context.db` bootstrap은 현재 schema를 항상 적용한다
- 저장소는 현재 schema에 필요한 additive migration만 수행한다
- replay와 derivation rebuild는 transaction 경계 안에서 search state까지 함께 갱신한다
- `project doctor`의 pending count는 `projection_state`, `derived_state`, `index_state`와 일치해야 한다
- canonical verification entrypoint는 [`../scripts/verify-release.sh`](../scripts/verify-release.sh) 다
