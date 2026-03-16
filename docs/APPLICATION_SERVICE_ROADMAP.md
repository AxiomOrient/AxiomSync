# Application Service Roadmap

## Intent

이 문서는 현재 `AxiomSync`에 직접 붙어 있는 `Resource/Search/Session/Runtime/ReleaseVerification`
책임을 **명시적 application service**로 올리는 실제 구현용 설계 문서다.

목표는 두 가지다.

- public API와 CLI 계약은 유지한다.
- 내부 구현은 `data-first`, `pure function 우선`, `side effect 후행` 구조로 재배치한다.

이 작업은 아키텍처 미화가 아니라 다음 문제를 줄이기 위한 것이다.

- `AxiomSync` 단일 impl에 책임이 계속 쌓이는 문제
- 테스트가 side effect에 묶여 pure logic 검증이 약해지는 문제
- CLI, runtime, release verification이 같은 receiver에 직접 결합되는 문제

## Scope

이 문서의 범위는 Phase 2 대상 서비스 다섯 개다.

- `RuntimeBootstrapService`
- `ResourceService`
- `SearchService`
- `SessionService`
- `ReleaseVerificationService`

이미 Phase 1로 분리된 서비스는 참고 기준만 제공한다.

- `EventService`
- `LinkService`
- `RepoService`
- `ArchiveService`

## Non-Goals

- public command 이름 변경
- JSON contract 변경
- SQLite schema 재설계
- retrieval backend 전환
- crate 분할

## Stable External Contract

이 리팩터링 동안 바꾸지 않는 외부 계약은 아래와 같다.

- CLI command surface
- public JSON field shape
- `AxiomSync` public entrypoint 이름
- SQLite schema key 이름
- process contract test 시나리오

즉 내부는 바꿔도 외부는 그대로여야 한다.

## Source Ownership Map

Phase 2에서 실제로 손대는 핵심 파일은 아래다.

- `src/client.rs`
- `src/client/facade.rs`
- `src/client/runtime.rs`
- `src/client/resource.rs`
- `src/client/search/*`
- `src/client/release/verify_service.rs`
- session orchestration이 직접 얽힌 `src/client/runtime.rs`, `src/client/search/*`

테스트 기준 파일은 아래다.

- `src/client/tests/*`
- `tests/process_contract.rs`
- `tests/repository_markdown_user_flows.rs`
- service 승격 후 추가할 pure/service unit test 파일

## Target Shape

최종 형태는 아래 원칙만 만족하면 된다.

1. `client.rs`는 composition root만 가진다.
2. `client/facade.rs`는 service accessor와 delegate만 가진다.
3. 각 service는 입력 데이터 정규화 -> pure planning -> side effect 실행 순서로 나뉜다.
4. pure planning 결과는 구조체로 남기고, side effect 함수는 그 구조체만 소비한다.
5. 테스트는 service 단위 검증 + process contract 검증으로 닫는다.

## Implementation Rules

리팩터링 patch는 아래 규칙을 지켜야 한다.

1. public delegate를 먼저 유지한다.
2. pure extraction을 먼저 하고 behavior change는 뒤로 미룬다.
3. 한 patch는 한 service만 다룬다.
4. side effect 함수는 계획 구조체 없이 직접 판단하지 않는다.
5. test는 service patch와 같은 change set에서 같이 닫는다.

## Service Contracts

### RuntimeBootstrapService

소유 책임:

- `bootstrap`
- `prepare_runtime`
- `initialize`
- `reindex_all`
- runtime restore / repair entry

입력:

- root path
- runtime config
- current state markers

순수 데이터 단계:

- `RuntimeBootstrapPlan`
- `RuntimeRestoreDecision`
- `IndexRepairPlan`

부수효과 단계:

- filesystem initialize
- schema ensure
- runtime index rebuild or restore
- repair run record append

완료 조건:

- `client.rs`에는 bootstrap 알고리즘이 남지 않는다.
- runtime 경로 테스트가 service 직접 호출로도 검증된다.

### ResourceService

소유 책임:

- `add_resource`
- `add_resource_with_ingest_options`
- staged ingest finalize
- wait/replay 정책 적용

입력:

- source
- target uri
- ingest options
- wait contract

순수 데이터 단계:

- `AddResourceIntent`
- `AddResourcePlan`
- `WaitStrategyDecision`

부수효과 단계:

- ingest staging
- manifest write
- finalize
- queue enqueue
- optional wait / replay

완료 조건:

- target URI 결정, wait mode 결정, request normalization은 pure function으로 이동한다.
- `resource.rs`의 main flow는 plan 생성과 plan 실행만 남는다.

### SearchService

소유 책임:

- `find`
- `search`
- `search_with_request`
- hint layering
- trace execution context 주입

입력:

- query
- target uri
- session
- runtime hints
- search filter / budget

순수 데이터 단계:

- `SearchIntent`
- `SearchPlan`
- `HintLayerPlan`
- `TraceContextPlan`

부수효과 단계:

- session snapshot read
- backend execution
- request log write
- trace persist

완료 조건:

- request normalization과 hint merge는 pure function만으로 검증 가능하다.
- backend 호출 전까지의 의사결정이 구조체로 노출된다.

### SessionService

소유 책임:

- `session(...)`
- session listing / delete
- promotion / archive-only checkpoint
- session scoped helper orchestration

입력:

- session id
- promotion request
- commit mode

순수 데이터 단계:

- `SessionScopePlan`
- `PromotionExecutionPlan`
- `SessionDeletionPlan`

부수효과 단계:

- session load/save
- promotion checkpoint CAS
- search/index cleanup

완료 조건:

- session 관련 판단 로직이 `Session` 객체 생성과 분리된다.
- `runtime.rs`와 `search/mod.rs`에서 session 조립 책임이 줄어든다.

### ReleaseVerificationService

소유 책임:

- `doctor_storage`
- `doctor_retrieval`
- `migrate_inspect`
- `migrate_apply`
- `release_verify`

입력:

- backup dir
- current schema markers
- retrieval backend status

순수 데이터 단계:

- `MigrationInspectPlan`
- `MigrationApplyPlan`
- `ReleaseVerifySnapshot`

부수효과 단계:

- backup file copy
- schema ensure
- migration run record append
- repair/migration/report read

완료 조건:

- verify path의 read-model 조립이 pure snapshot builder로 이동한다.
- handler는 service 결과만 출력한다.

## Simplified Build Method

기존의 `intent -> spec -> prompt plan -> task chunking -> iterative execution -> test loop -> patch`
를 구현 관점에서 더 단순하게 줄이면 아래 네 단계면 충분하다.

1. `Boundary freeze`
   - public 함수 시그니처와 JSON contract를 고정한다.
   - 이동 대상 로직과 남길 로직을 파일별로 자른다.
2. `Data extraction`
   - 판단 로직을 구조체 기반 pure function으로 먼저 뺀다.
   - 이 단계에서는 side effect 코드를 건드리지 않는다.
3. `Delegate insertion`
   - facade accessor를 추가하고 service가 pure plan + execute를 갖게 한다.
   - 기존 public 메서드는 delegate만 하게 줄인다.
4. `Verification close`
   - pure unit test
   - service integration test
   - process contract test
   - release gate 실행

핵심은 “service struct를 먼저 만들고 그 안에서 다시 모든 것을 하자”가 아니다.
핵심은 **판단 로직을 먼저 data로 만들고, 그 다음 service가 그 data를 실행하게 하자**이다.

## Recommended Execution Order

순서는 결합도가 낮고 검증이 쉬운 쪽부터 간다.

1. `ReleaseVerificationService`
2. `RuntimeBootstrapService`
3. `ResourceService`
4. `SearchService`
5. `SessionService`

이 순서를 권장하는 이유:

- release/runtime은 입력과 출력이 비교적 고정돼 있어 pure snapshot 분리가 쉽다.
- resource는 ingest/wait 경계를 자르기 좋다.
- search와 session은 상호 의존이 강해서 뒤로 미는 편이 안전하다.

## Patch Strategy

한 번에 하나의 서비스만 옮긴다.

각 patch는 아래를 반드시 같이 낸다.

- service struct 추가
- facade delegate 추가
- pure planning struct 추가
- 기존 public API 유지
- 회귀 테스트 추가 또는 기존 테스트 이동

금지:

- 여러 서비스 동시 이동
- public command/JSON contract 변경
- pure extraction과 behavior change를 같은 patch에 섞기

## Verification Loop

서비스 하나를 옮길 때마다 아래 루프를 돈다.

1. pure function test
2. service unit/integration test
3. 관련 process contract 재실행
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test -p axiomsync`

release 전 최종 루프:

- `cargo audit --deny unsound --deny unmaintained --deny yanked`
- `bash -n scripts/*.sh`
- `cargo test -p axiomsync`

## Task Chunks

### Chunk A: ReleaseVerificationService

Done when:

- verify_service 내부 read-model 조립이 `Plan/Snapshot` 구조체로 분리된다.
- `doctor` / `migrate` / `release verify` handler는 출력만 한다.

### Chunk B: RuntimeBootstrapService

Done when:

- runtime prepare/reindex/restore 판단이 pure decision 구조체로 분리된다.
- `client.rs`는 wiring과 root state 보관만 한다.

### Chunk C: ResourceService

Done when:

- target resolve, wait decision, ingest finalize mode가 pure planning 단계에 있다.
- resource add path의 side effect는 execute 함수에만 남는다.

### Chunk D: SearchService

Done when:

- request normalization, hint layering, trace context 조립이 pure struct 기반으로 정리된다.
- backend 호출 이전 판단 흐름이 테스트 가능하다.

### Chunk E: SessionService

Done when:

- session creation/promotion/deletion orchestration이 service로 승격된다.
- runtime/search가 session helper를 직접 많이 알지 않아도 된다.

## Final Done Condition

이 문서 기준 Phase 2가 완료됐다고 말하려면 아래가 모두 필요하다.

- `BLUEPRINT.md`의 Application Layer 서비스가 코드에서 모두 확인된다.
- `client/facade.rs`는 delegate만 가진다.
- `client.rs`는 composition root만 가진다.
- 각 service에 최소 하나 이상의 pure planning data 구조체가 있다.
- `cargo clippy --workspace --all-targets -- -D warnings` 통과
- `cargo test -p axiomsync` 통과
- process contract 회귀 통과
