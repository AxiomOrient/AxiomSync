# Implementation Plan

## Goal
최신 코드 기준으로 문서 체계를 정리하고, 대형 모듈을 점진적으로 분해해 유지보수성을 높인다.

## Scope
- 문서 canonical/archive 구조 재정렬
- 날짜/버전/개인 절대경로 노이즈 제거
- 릴리즈 signoff 문서/스크립트 stable 파일명 정렬
- `axiomme-core/src/index.rs` 1차/2차 분해 (`exact`, `rank`)
- `axiomme-core/src/session/commit` 단계 분해 (`types`, `promotion`, `dedup`, `write_path`, `read_path`, `apply_flow`, `apply_modes`, `fallbacks`)
- `axiomme-core/src/release_gate.rs` 단계 분해 (`workspace_command`, `episodic_semver`, `contract_probe`, `policy`, `build_quality`, `workspace`)
- `axiomme-core/src/index.rs` 후속 분해 (`filter`, `ancestry`, `lifecycle`, `search_flow`, `text_assembly` 경계 완료)
- `axiomme-core/src/release_gate.rs` 후속 분해 (`decision`, `test_support`, `tests`, `contract_integrity` 경계 완료)

## Verification Map
1. Narrow
- `cargo fmt --all`
- `cargo check -p axiomme-core --lib`
- `cargo test -p axiomme-core --lib index::tests::`

2. Medium
- `cargo test -p axiomme-core --lib`
- `cargo check --workspace --all-targets`

3. Broad
- `bash scripts/quality_gates.sh`

## Workstreams
1. Documentation normalization
- canonical 문서 재작성
- archive 이관
- README/크레이트 README 정합성 갱신

2. Script/document consistency
- signoff 스크립트 기본 문서 경로를 stable 이름으로 정렬
- 절대경로 기본값 제거

3. Large module decomposition
- `index.rs` exact-match 관련 로직을 `index/exact.rs`로 분리
- `index.rs` 랭킹/스코어링 헬퍼를 `index/rank.rs`로 분리
- `session/commit/mod.rs`의 데이터 모델/상수 경계를 `session/commit/types.rs`로 분리
- promotion 보조 함수를 `session/commit/promotion.rs`로 분리
- dedup/LLM selection 경계를 `session/commit/dedup.rs`로 분리
- memory write side-effect 경계를 `session/commit/write_path.rs`로 분리
- memory read/list 경계를 `session/commit/read_path.rs`로 분리
- promotion checkpoint/apply 오케스트레이션 경계를 `session/commit/apply_flow.rs`로 분리
- promotion apply mode 핸들러 경계를 `session/commit/apply_modes.rs`로 분리
- memory fallback 기록 경계를 `session/commit/fallbacks.rs`로 분리
- release gate workspace command side-effect 경계를 `release_gate/workspace_command.rs`로 분리
- release gate episodic semver 파싱/판정 경계를 `release_gate/episodic_semver.rs`로 분리
- release gate contract probe 실행 경계를 `release_gate/contract_probe.rs`로 분리
- release gate policy 경계를 `release_gate/policy.rs`로 분리
- release gate build-quality 경계를 `release_gate/build_quality.rs`로 분리
- release gate workspace 경계를 `release_gate/workspace.rs`로 분리
- index filter 경계(`normalize_filter`, `leaf_matches_filter`, `record_matches_filter`)를 `index/filter.rs`로 분리
- index ancestry/filter projection 경계(`has_matching_leaf_descendant`, `filter_projection_uris`)를 `index/ancestry.rs`로 분리
- index lifecycle mutation 경계(`remove_lexical_stats`, `upsert_child_index_entry`, `remove_child_index_entry`)를 `index/lifecycle.rs`로 분리
- index search orchestration 경계(`search`, `search_directories`)를 `index/search_flow.rs`로 분리
- index text assembly/helper 경계(`build_upsert_text`)를 `index/text_assembly.rs`로 분리
- release gate decision constructor 경계(`gate_decision`, `*_gate_decision`)를 `release_gate/decision.rs`로 분리
- release gate 테스트 픽스처 경계(`eval_report`, `benchmark_gate_result`, `write_contract_gate_workspace_fixture`)를 `release_gate/test_support.rs`로 분리
- release gate inline 통합 테스트 모듈(`mod tests`)을 `release_gate/tests.rs` 파일로 분리
- release gate contract-integrity 오케스트레이션 경계(`evaluate_contract_integrity_gate`)를 `release_gate/contract_integrity.rs`로 분리
- 동작 변경 없이 컴파일/테스트 동등성 확보

## Exit Criteria
- canonical 문서에 불필요한 날짜/버전/개인 경로가 없음
- 분해 후 테스트와 품질 게이트 통과
- `docs/TASKS.md` 상태/증거 동기화 완료

## Progress
- 완료: 문서 canonical/archive 정리 및 stable signoff 문서명 정렬
- 완료: `index.rs` exact-match 블록을 `index/exact.rs`로 1차 분해
- 완료: `index.rs` 랭킹/스코어링 블록을 `index/rank.rs`로 2차 분해
- 완료: `session/commit` data model/constants를 `types.rs`로 분리하고 promotion helper를 `promotion.rs`로 분리
- 완료: `session/commit` dedup/LLM selection을 `dedup.rs`로 분리
- 완료: `session/commit` write path(`persist_memory`, `persist_promotion_candidate`, `reindex_memory_uris`)를 `write_path.rs`로 분리
- 완료: `session/commit` read path(`list_existing_*`, `list_memory_document_uris`)를 `read_path.rs`로 분리
- 완료: `session/commit` promotion checkpoint/apply 오케스트레이션을 `apply_flow.rs`로 분리
- 완료: `session/commit` promotion apply mode(`all_or_nothing`, `best_effort`) 핸들러를 `apply_modes.rs`로 분리
- 완료: `session/commit` memory fallback 기록 경계를 `fallbacks.rs`로 분리
- 완료: `release_gate` workspace command 실행/모킹 경계를 `release_gate/workspace_command.rs`로 분리
- 완료: `release_gate` episodic semver 파싱/판정 경계를 `release_gate/episodic_semver.rs`로 분리
- 완료: `release_gate` contract probe 실행 경계를 `release_gate/contract_probe.rs`로 분리
- 완료: `release_gate` policy 경계를 `release_gate/policy.rs`로 분리
- 완료: `release_gate` build-quality 경계를 `release_gate/build_quality.rs`로 분리
- 완료: `release_gate` workspace 경계(`resolve_workspace_dir`)를 `release_gate/workspace.rs`로 분리하고 경로 검증 회귀 테스트 3건 추가
- 완료: `index` filter 경계(`normalize_filter`, `leaf_matches_filter`, `record_matches_filter`)를 `index/filter.rs`로 분리
- 완료: `index` ancestry/filter projection 경계(`has_matching_leaf_descendant`, `filter_projection_uris`)를 `index/ancestry.rs`로 분리
- 완료: `index` lifecycle mutation 경계(`remove_lexical_stats`, `upsert_child_index_entry`, `remove_child_index_entry`)를 `index/lifecycle.rs`로 분리하고 `index.rs` LOC를 1561 -> 1515로 축소
- 완료: `index` search orchestration 경계(`search`, `search_directories`)를 `index/search_flow.rs`로 분리하고 `index.rs` LOC를 1515 -> 1391로 축소
- 완료: `index` text assembly/helper 경계(`build_upsert_text`)를 `index/text_assembly.rs`로 분리하고 `index.rs` LOC를 1391 -> 1367로 축소
- 완료: `release_gate` decision constructor 경계(`gate_decision`, `*_gate_decision`, `finalize_release_gate_pack_report`)를 `release_gate/decision.rs`로 분리
- 완료: `release_gate` 테스트 픽스처 헬퍼 경계(`eval_report`, `benchmark_gate_result`, `write_contract_gate_workspace_fixture`)를 `release_gate/test_support.rs`로 분리하고 `release_gate.rs` LOC를 993 -> 878로 축소
- 완료: `release_gate` inline 통합 테스트 모듈(`mod tests`)을 `release_gate/tests.rs`로 분리하고 `release_gate.rs` LOC를 878 -> 269로 축소
- 완료: `release_gate` contract-integrity 오케스트레이션 경계(`evaluate_contract_integrity_gate`)를 `release_gate/contract_integrity.rs`로 분리하고 `release_gate.rs` LOC를 269 -> 245로 축소
- 완료: 검증 단계에서 발생한 clippy gate drift(`manual_is_multiple_of`)를 `retrieval/expansion.rs`에서 표준 API(`is_multiple_of`)로 정합화해 quality gates 복구
- 완료: 검증 단계에서 발생한 formatting drift(`release_gate/tests.rs` 선행 공백 1라인)를 정리해 quality gates 복구
- 완료: 동시 변경으로 깨진 `session/commit` import 경로와 테스트 문자열 리터럴을 정합화해 `axiomme-core` 컴파일/테스트 게이트 복구
- 검증: `cargo fmt --all`, `cargo check -p axiomme-core --lib`, `cargo test -p axiomme-core --lib session::commit::tests::memory_dedup_mode_defaults_to_auto`, `cargo test -p axiomme-core --lib session::commit::tests::resolve_merge_target_index_requires_valid_target` 통과
- 검증 확장: `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` 재통과
