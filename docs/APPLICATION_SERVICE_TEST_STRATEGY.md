# Application Service Test Strategy

## Intent

이 문서는 application service 전면 리팩터링 동안
"무엇을 왜 테스트해야 하는지"를 고정한다.

핵심 목표는 세 가지다.

- pure planning 로직이 side effect 없이 검증되도록 만든다.
- service 승격 중 public contract 회귀를 막는다.
- process contract와 release gate로 최종 제품 동작을 닫는다.

## Coverage Model

테스트는 아래 네 층으로 분리한다.

1. `Pure planning test`
   - intent normalization
   - decision / plan builder
   - branching / fallback choice
2. `Service unit/integration test`
   - service가 store/fs/index를 올바르게 조합하는지 검증
3. `Process contract test`
   - CLI/boot/runtime/archive/release 흐름 회귀 방지
4. `Release gate`
   - clippy / cargo test / cargo audit

## What Must Not Regress

- canonical JSON output
- `--compat-json` opt-in 동작
- `doctor` / `migrate` / `release verify` JSON contract
- clean-root init -> ingest -> search -> event -> archive -> release verify flow
- mixed intent / FTS fallback trace evidence
- repository markdown user flow

## Service Coverage Matrix

| Service | Pure Planning Coverage | Service Coverage | Process/E2E Coverage | Notes |
|---|---|---|---|---|
| RuntimeBootstrapService | restore decision, reindex decision, repair decision | prepare/init/reindex orchestration | clean-root init, release verify | runtime marker drift와 reindex trigger를 증명 |
| ResourceService | target resolve, wait decision, ingest finalize mode | add/wait/replay orchestration | add -> search visibility | wait strict/relaxed 회귀 방지 |
| SearchService | request normalization, hint layering, trace context plan | backend call + trace persist + request log | mixed intent, drift fallback, real docs flow | search는 가장 회귀 위험이 큼 |
| SessionService | session scope plan, promotion plan, delete cleanup plan | promotion/delete/archive-only orchestration | session-aware search and promotion scenarios | search/runtime 결합면 주의 |
| ReleaseVerificationService | inspect/apply/verify snapshot builder | backup/schema ensure/report assembly | doctor/migrate/release verify CLI flow | JSON payload는 fixture까지 닫기 |

## Minimal Test Files To Maintain

기존 파일은 가능한 재사용한다.

- `crates/axiomsync/src/client/tests/*`
- `crates/axiomsync/src/client/search/backend_tests.rs`
- `crates/axiomsync/tests/process_contract.rs`
- `crates/axiomsync/tests/repository_markdown_user_flows.rs`
- `crates/axiomsync/tests/core_contract_fixture.rs`
- `crates/axiomsync/tests/release_contract_fixture.rs`

새 테스트 파일은 아래 원칙으로만 추가한다.

- service 하나당 pure planning test 파일 1개
- service 하나당 orchestration test 파일 1개 이하
- process contract는 공용 파일에 유지

## Expected Test Additions By Phase

### Phase A: ReleaseVerificationService

추가 테스트:

- inspect/apply/verify snapshot builder pure test
- backup path / migration audit trail orchestration test

이미 유지할 테스트:

- release contract fixture
- process contract의 doctor/migrate/release verify flow

### Phase B: RuntimeBootstrapService

추가 테스트:

- runtime restore decision pure test
- index repair plan pure test

이미 유지할 테스트:

- initialization lifecycle tests
- queue reconcile drift/reindex tests

### Phase C: ResourceService

추가 테스트:

- target resolution pure test
- wait strategy decision pure test

이미 유지할 테스트:

- resource add/wait tests
- search visibility tests

### Phase D: SearchService

추가 테스트:

- request normalization pure test
- hint layer plan pure test
- trace context plan pure test

이미 유지할 테스트:

- `backend_tests.rs` mixed intent / fts fallback
- repository markdown user flow

### Phase E: SessionService

추가 테스트:

- session scope plan pure test
- promotion execution plan pure test
- session deletion cleanup plan pure test

이미 유지할 테스트:

- session tests
- search/session interaction tests

## Gate Commands

서비스 patch마다 최소로 돌릴 명령:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p axiomsync`

릴리즈 판단 전 최종 명령:

- `cargo audit --deny unsound --deny unmaintained --deny yanked`
- `cargo test -p axiomsync`

## Done Criteria

리팩터링이 완료됐다고 말하려면 아래가 필요하다.

- 각 Phase 2 service에 pure planning test가 존재한다.
- 각 service 승격 patch가 기존 process contract를 깨지 않는다.
- release fixture와 core fixture가 유지된다.
- repository markdown user flow가 유지된다.
- 최종 gate 명령이 모두 통과한다.
