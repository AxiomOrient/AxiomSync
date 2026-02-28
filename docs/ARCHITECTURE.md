# Architecture

## Project Intent
AxiomMe는 컨텍스트 데이터를 로컬 파일시스템/SQLite/인메모리 인덱스로 일관되게 다루는 런타임입니다.

## System Context
주요 사용자/환경:
- 개발자: CLI로 인제스트/검색/세션 작업 수행
- 운영자/CI: 품질 게이트와 릴리즈 게이트 검증 수행
- 모바일 통합자: FFI 경계를 통해 런타임 기능 호출

외부 의존:
- Rust crate 생태계
- `cargo-audit` advisory DB
- 선택적 외부 웹 뷰어 바이너리(`axiomme-webd` 또는 `AXIOMME_WEB_VIEWER_BIN`)

## Architecture Layers
1. Interface Layer
- `axiomme-cli`
- `axiomme-mobile-ffi`

2. Application Layer
- `axiomme-core::client` (resource/search/session/release orchestration)

3. Domain & Storage Layer
- URI/모델: `uri`, `models`
- 파일시스템: `fs`
- 상태 저장소: `state` (SQLite)
- 검색/인덱스: `index`, `retrieval`
- 세션/메모리: `session`

## Data Flow
1. `add_resource`로 리소스 스테이징
2. outbox/queue 이벤트 적재
3. replay/reconcile로 처리
4. 인덱스/검색 문서 갱신
5. `find/search`로 랭킹 결과 반환

## Boundary Rules
- Canonical URI: `axiom://{scope}/{path}`
- `queue` scope는 비시스템 쓰기 금지
- side effect는 filesystem/state/network/host tools로 명시 분리
- host tool 실행은 정책 게이트(`AXIOMME_HOST_TOOLS`)를 따름

## Quality & Release Gates
- 품질 게이트: `scripts/quality_gates.sh`
- strict 릴리즈 게이트: `scripts/release_pack_strict_gate.sh`
- 사인오프 상태: `scripts/release_signoff_status.sh`

## Current Decomposition Focus
대형 모듈 분해 우선순위:
1. `crates/axiomme-core/src/index.rs` (핵심 검색/업서트 기능 경계 분해 완료 상태 유지, 잔여 보조 경계 재평가)
2. `crates/axiomme-core/src/release_gate.rs` (핵심 오케스트레이션 경계 분해 완료 상태 유지, 잔여 wrapper 단위 재평가)

완료된 분해:
- `crates/axiomme-core/src/index.rs` -> `index/exact.rs`(exact-match), `index/rank.rs`(랭킹 헬퍼)로 분리
- `crates/axiomme-core/src/index.rs` -> `index/filter.rs`(필터 정규화/판정 경계)로 분리
- `crates/axiomme-core/src/index.rs` -> `index/ancestry.rs`(ancestor/filter projection 트리 탐색 경계)로 분리
- `crates/axiomme-core/src/index.rs` -> `index/lifecycle.rs`(lexical/docfreq + child-index mutation 경계)로 분리
- `crates/axiomme-core/src/index.rs` -> `index/search_flow.rs`(search/search_directories 오케스트레이션 경계)로 분리
- `crates/axiomme-core/src/index.rs` -> `index/text_assembly.rs`(upsert payload text assembly/helper 경계)로 분리
- `index.rs`는 검색 오케스트레이션과 인덱스 수명주기 중심으로 축소
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/types.rs`(데이터 모델/상수), `session/commit/promotion.rs`(promotion 보조 함수)로 1차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/dedup.rs`(dedup/LLM selection 경계)로 2차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/write_path.rs`(persist/reindex side effect)로 3차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/read_path.rs`(memory read/list 경계)로 4차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/apply_flow.rs`(promotion checkpoint/apply 오케스트레이션 경계)로 5차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/apply_modes.rs`(promotion apply mode 핸들러 경계)로 6차 분리
- `crates/axiomme-core/src/session/commit/mod.rs` -> `session/commit/fallbacks.rs`(memory fallback 기록 경계)로 7차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/workspace_command.rs`(workspace command 실행/모킹 side-effect 경계)로 1차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/episodic_semver.rs`(episodic semver 파싱/정책 경계)로 2차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/contract_probe.rs`(contract probe 실행 경계)로 3차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/policy.rs`(contract policy 생성 경계)로 4차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/build_quality.rs`(build-quality 실행/요약 경계)로 5차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/workspace.rs`(workspace 경로 검증/정규화 경계)로 6차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/decision.rs`(gate decision constructor 경계)로 7차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/test_support.rs`(테스트 픽스처 helper 경계)로 8차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/tests.rs`(inline 통합 테스트 파일 경계)로 9차 분리
- `crates/axiomme-core/src/release_gate.rs` -> `release_gate/contract_integrity.rs`(contract-integrity probe aggregation 오케스트레이션 경계)로 10차 분리
