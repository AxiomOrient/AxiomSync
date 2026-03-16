# Application Service Full Refactor Plan

## Mission

`AxiomSync`의 남은 Phase 2 책임을 explicit application service로 승격한다.

범위:

- `RuntimeBootstrapService`
- `ResourceService`
- `SearchService`
- `SessionService`
- `ReleaseVerificationService`

고정 조건:

- public CLI/API 계약 유지
- JSON payload shape 유지
- SQLite schema/key 유지
- process contract 유지

## Execution Order

1. `ReleaseVerificationService`
2. `RuntimeBootstrapService`
3. `ResourceService`
4. `SearchService`
5. `SessionService`

## Phase Strategy

각 phase는 같은 패턴으로 진행한다.

1. facade delegate 유지
2. pure planning data 구조체 추출
3. service execute 경로 도입
4. 기존 public method를 delegate-only로 축소
5. 관련 테스트 추가 및 회귀 검증

## Deliverables

- explicit service struct
- pure planning structs
- service-level tests
- updated docs only when contract/ownership 설명이 바뀔 때

## Global Done Condition

- Phase 2 서비스 다섯 개가 코드에서 확인된다.
- `client.rs`는 composition root만 가진다.
- `client/facade.rs`는 accessor/delegate만 가진다.
- pure planning test + service test + process contract가 유지된다.
- `cargo clippy --workspace --all-targets -- -D warnings` 통과
- `cargo test -p axiomsync` 통과
- `cargo audit --deny unsound --deny unmaintained --deny yanked` 통과
