# Final-Form Conformance Review

기준: `axiomsync-final-form-docs-package`

## Summary

현재 레포는 새 docs package 기준으로 `정합`이다.

- final-form fixture는 실제 회귀 테스트에 연결된다
- sink, HTTP, MCP, auth, pending counts는 active implementation과 package가 같은 계약을 쓴다
- package는 과거 제안이 아니라 현재 shipping truth를 기준으로 다시 작성됐다

## Proof

- fixture schema validation: [final_form_compat.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/final_form_compat.rs#L189)
- example ingest/rebuild/query: [final_form_compat.rs](/Users/axient/repository/AxiomSync/crates/axiomsync/tests/final_form_compat.rs#L28)
- active HTTP router: [lib.rs](/Users/axient/repository/AxiomSync/crates/axiomsync-http/src/lib.rs#L26)
- active sink docs: [README.md](/Users/axient/repository/AxiomSync/README.md#L18)
- storage truth: [schema.sql](/Users/axient/repository/AxiomSync/crates/axiomsync-store-sqlite/src/schema.sql#L1)

## What Changed

- package는 단일 `append_raw_events()` fantasy API를 버리고 실제 `plan/apply` sink contract를 기준으로 잡는다
- package는 `raw_events/source_cursors` rename proposal을 버리고 shipping schema names를 그대로 쓴다
- package는 `claims`를 숨기지 않고 current derived helper로 문서화한다
- package는 MCP tool name을 namespaced 예시가 아니라 실제 wire name으로 적는다

## Residual Risk

남은 리스크는 active runtime drift보다 historical artifact drift다.

- `axiomsync-b8e8828-audit-patch-package` 같은 감사 산출물은 참고 자료로만 읽어야 한다
- 삭제 예정이거나 stale인 문서 묶음은 release contract로 재사용하면 안 된다

## Final Assessment

새 docs package는 현재 릴리스의 final-form contract로 사용 가능하다.
