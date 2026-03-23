# Final-Form Conformance Review

기준: `axiomsync-final-form-docs-package`

판정 기준:
- active entrypoint가 docs package 계약을 만족하면 `개선됨`
- stale 코드/문서가 active 경로와 충돌하면 `부분 개선, 정합성 미완료`
- public contract, 저장 책임, data flow, auth/scope, determinism에 영향을 주는 차이만 finding으로 기록

## Summary

현재 레포는 docs package 방향으로 **부분 개선**됐다.

- final-form fixture는 실제 회귀 테스트에 연결돼 있다.
- active sink/HTTP/MCP surface는 final-form envelope, evidence-first derivation, pending counts를 실제로 검증한다.
- 하지만 docs package가 의도한 최종형과 아직 어긋나는 active implementation drift가 남아 있다.
- repo 내부에는 stale 문서/병렬 경로가 남아 있어 정합성이 완전히 닫히지 않았다.

## Improvement Proof

- fixture schema validation: [crates/axiomsync/tests/final_form_compat.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/final_form_compat.rs#L196)
- final-form example ingest/rebuild/query: [crates/axiomsync/tests/final_form_compat.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/final_form_compat.rs#L29)
- HTTP/MCP parity: [crates/axiomsync/tests/http_and_mcp_v2.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/http_and_mcp_v2.rs)
- active HTTP router: [crates/axiomsync-http/src/lib.rs](/Users/axient/repository/AxiomSync/crates/axiomsync-http/src/lib.rs#L26)
- storage truth: [crates/axiomsync-store-sqlite/src/schema.sql](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L1)

## Findings

### [P2] `claim` 제거 결정이 active runtime까지 반영되지 않음

- Docs package 근거:
  [axiomsync-final-form-docs-package/08_CONSISTENCY_REVIEW.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/08_CONSISTENCY_REVIEW.md#L35) 는 `claim`을 제거하고 `insight`로 통합한다고 명시한다.
- 현재 구현 근거:
  [docs/API_CONTRACT.md](/Users/axient/repository/AxiomSync/docs/API_CONTRACT.md#L37) 는 `search-claims`, `get-claim`, `/api/claims/{id}`를 canonical read surface에 포함한다.
  [crates/axiomsync-http/src/lib.rs](/Users/axient/repository/AxiomSync/crates/axiomsync-http/src/lib.rs#L40) 는 `/api/claims/{id}`와 `/api/query/search-claims`를 active router에 노출한다.
  [crates/axiomsync-store-sqlite/src/schema.sql](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L142) 는 `claims`, `claim_evidence` 테이블을 유지한다.
- 판정:
  fixture는 통과하지만 docs package가 의도한 derived-memory 단순화는 아직 구현되지 않았다.
- Active runtime 영향:
  있음. public contract와 DB schema에 그대로 노출된다.

### [P2] ingest public contract가 docs package의 `append_raw_events` 모델과 다름

- Docs package 근거:
  [axiomsync-final-form-docs-package/04_API_AND_MCP_SPEC.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/04_API_AND_MCP_SPEC.md#L3) 는 write API를 `append_raw_events(batch)`, `upsert_source_cursor(cursor)`, `health()`로 정의한다.
  같은 문서는 `accepted[]` / `rejected[]` 응답을 제시한다([response](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/04_API_AND_MCP_SPEC.md#L52)).
- 현재 구현 근거:
  [README.md](/Users/axient/repository/AxiomSync/README.md#L20) 와 [docs/API_CONTRACT.md](/Users/axient/repository/AxiomSync/docs/API_CONTRACT.md#L61) 는 `/sink/raw-events/plan`, `/sink/raw-events/apply`, `/sink/source-cursors/plan`, `/sink/source-cursors/apply` 4-endpoint를 정본으로 둔다.
  [crates/axiomsync-http/src/lib.rs](/Users/axient/repository/AxiomSync/crates/axiomsync-http/src/lib.rs#L28) 도 같은 4-endpoint를 active router에 노출한다.
  [crates/axiomsync/tests/final_form_compat.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/final_form_compat.rs#L38) 는 final-form fixture를 `plan_append_raw_events -> apply_ingest_plan` 흐름으로 검증한다.
- 판정:
  final-form envelope 수용과 plan/apply 분리는 구현됐지만, docs package의 public ingest shape까지는 수렴하지 않았다.
- Active runtime 영향:
  있음. external writer가 보는 write contract가 docs package 예시와 다르다.

### [P2] storage model이 docs package의 `raw_events/source_cursors + stable_key` 구조와 다름

- Docs package 근거:
  [axiomsync-final-form-docs-package/03_STORAGE_SCHEMA.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/03_STORAGE_SCHEMA.md#L5) 는 raw ledger를 `raw_events`, cursor를 `source_cursors`로 정의한다.
  [axiomsync-final-form-docs-package/schema/axiomsync_kernel_vnext.sql](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/schema/axiomsync_kernel_vnext.sql#L4) 는 `stable_key` 중심 테이블 골격을 제시한다.
- 현재 구현 근거:
  [crates/axiomsync-store-sqlite/src/schema.sql](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L4) 는 raw ledger를 `ingress_receipts`, cursor를 `source_cursor`로 둔다.
  같은 스키마의 [sessions](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L41), [entries](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L65) 에는 docs package가 강조한 `stable_key`가 없다.
- 판정:
  역할은 유사하지만 docs package가 제안한 저장 모델과 동일하다고 보기는 어렵다.
- Active runtime 영향:
  있음. storage truth와 replay 출발점 명칭이 docs package와 다르다.

### [P3] repo 문서에 stale architecture 설명이 남아 있음

- Docs package 근거:
  [axiomsync-final-form-docs-package/01_FINAL_FORM.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/01_FINAL_FORM.md#L15) 와 [03_STORAGE_SCHEMA.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/03_STORAGE_SCHEMA.md#L10) 는 `session / entry / artifact / anchor` 중심과 `episodes / insights / verifications / procedures`를 최종형으로 둔다.
- 현재 구현 근거:
  [docs/RUNTIME_ARCHITECTURE.md](/Users/axient/repository/AxiomSync/docs/RUNTIME_ARCHITECTURE.md#L21) 는 `raw_event`, `conv_session`, `conv_turn`, `search_doc_redacted`, canonical noun `case`를 말한다.
  반면 active release docs와 runtime은 [README.md](/Users/axient/repository/AxiomSync/README.md#L64), [docs/API_CONTRACT.md](/Users/axient/repository/AxiomSync/docs/API_CONTRACT.md#L104), [crates/axiomsync-http/src/lib.rs](/Users/axient/repository/AxiomSync/crates/axiomsync-http/src/lib.rs#L36) 기준으로 `session/entry/artifact/anchor/episode/insight/procedure`를 정본으로 둔다.
- 판정:
  구현보다 문서가 stale 하다.
- Active runtime 영향:
  직접 오동작은 없지만, 유지보수와 리뷰 기준을 흐린다.

### [P3] MCP naming contract가 docs package와 다름

- Docs package 근거:
  [axiomsync-final-form-docs-package/04_API_AND_MCP_SPEC.md](/Users/axient/repository/AxiomSync/axiomsync-final-form-docs-package/04_API_AND_MCP_SPEC.md#L129) 는 `axiomsync.search_insights` 같은 namespaced tool 예시를 든다.
- 현재 구현 근거:
  [crates/axiomsync/tests/http_and_mcp_v2.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/http_and_mcp_v2.rs#L453) 는 active tool names를 `search_docs`, `search_insights`, `find_fix`, `get_evidence_bundle`, `get_task`로 검증한다.
- 판정:
  read-only 원칙은 지켜졌지만 wire-level naming은 docs package와 다르다.
- Active runtime 영향:
  있음. MCP client가 보는 tool 이름이 docs package 예시와 다르다.

## Checks Completed

- Sink / ingest contract
  final-form envelope 수용, schema validation, plan/apply 분리, loopback-only write는 구현됨.
  docs package의 단일 `append_raw_events` public contract는 미구현.
- Query / HTTP / MCP surface
  `search_docs`, evidence bundle, auth boundary, compatibility alias는 active tests로 검증됨.
  `claim` canonical status와 MCP naming은 docs package와 불일치.
- Storage / projection / derivation model
  evidence-first derivation, rebuildability, pending counts는 구현됨.
  `raw_events/source_cursors + stable_key` 저장 모델은 미구현.
- Parallel stack drift
  stale repo docs가 남아 있고 canonical noun 설명이 충돌함.
- Improvement proof
  docs package fixture는 실제 회귀 테스트에 연결돼 있음.
  따라서 fixture-only 반영을 넘었지만, 아키텍처 최종형까지 완전히 수렴한 것은 아님.

## Final Assessment

최종 판정은 `부분 개선, 정합성 미완료`다.

- `개선됨`:
  final-form fixture 연결, evidence-first derivation, `search_docs`, HTTP/MCP parity, pending counts, loopback-only sink
- `미완료`:
  `claim` 제거, docs package 기준 ingest public contract, docs package 저장 모델 수렴, stale 문서 정리

남은 리스크 분류:
- `future drift risk`:
  active runtime과 repo 문서의 canonical noun/architecture 설명 충돌
- `stale artifact only`:
  없음. 현재는 docs drift와 active contract drift가 모두 남아 있다.
