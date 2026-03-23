# 06. 구현 로드맵

## Phase 0 — 계약 고정

목표:
- raw ingest / query / MCP boundary 확정
- entity naming 확정
- SQL skeleton 확정

완료 기준:
- `kernel_sink_contract.json` frozen
- core table set frozen
- write ownership 표 frozen

## Phase 1 — raw ledger

구현:
- `raw_events`
- `source_cursors`
- dedupe
- accept/reject response
- health endpoint

완료 기준:
- duplicate replay가 safe
- append_raw_events는 raw accept만 보장
- projection failure와 raw accept가 분리됨

## Phase 2 — canonical projection

구현:
- sessions
- actors
- entries
- artifacts
- anchors
- replay from raw

완료 기준:
- ChatGPT selection packet이 session/entry/anchor로 투영됨
- Rams run summary가 session/entry/artifact로 투영됨
- rebuild projection이 deterministic

## Phase 3 — derived memory

구현:
- episodes
- insights
- verifications
- procedures
- evidence links

완료 기준:
- 최소 3개 질의가 evidence-backed로 답변됨
  - `find_fix`
  - `find_decision`
  - `find_runbook`
- evidence 없는 reusable record 생성 차단

## Phase 4 — retrieval surface

구현:
- `search_docs`
- `search_docs_fts`
- query API
- MCP read tools/resources

완료 기준:
- entry / episode / insight / procedure 검색 가능
- retrieval answer에 anchor preview 포함
- MCP에서 read-only 사용 가능

## Phase 5 — integration

구현:
- AxiomRelay adapter
- axiomRams adapter
- example payload fixtures
- import contract tests

완료 기준:
- Relay packet fixture 통과
- Rams packet fixture 통과
- cursor update fixture 통과

## Phase 6 — maintenance / rebuild

구현:
- replay
- rebuild projection
- rebuild derived
- rebuild index
- purge policy
- repair tooling

완료 기준:
- DB 삭제 후 raw backup으로 복구 가능
- index drop/rebuild 안전
- corrupted derived layer 재생성 가능

## Phase 7 — hardening

구현:
- migration tests
- state invariant tests
- idempotency tests
- crash/restart tests
- performance baseline

완료 기준:
- duplicate ingest property test 통과
- rebuild determinism test 통과
- 1인 로컬 사용 규모에서 latency 허용 범위 확보

## 우선순위 원칙

1. raw accept correctness
2. canonical projection correctness
3. evidence anchoring correctness
4. query usefulness
5. convenience features

## 미루어도 되는 것

- embeddings
- graph expansion
- cross-device sync
- semantic reranker
- UI polish

이건 core가 아니다.
