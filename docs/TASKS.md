# Tasks

## Task Table
| TASK-ID | Status | Priority | Source | Action | Evidence |
| --- | --- | --- | --- | --- | --- |
| TASK-030 | DONE | P0 | user | 문서 canonical/archive 재정렬, 날짜/절대경로 노이즈 제거, 최신 코드 기준 문서 정합성 갱신 | `README.md`, `docs/README.md`, `docs/ARCHITECTURE.md`, `docs/FEATURE_COMPLETENESS_UAT_GATE.md`, `docs/RELEASE_SIGNOFF_*.md`, `docs/archive/*` |
| TASK-031 | DONE | P0 | user | `crates/axiomme-core/src/index.rs` 대형 모듈 1차 분해(exact-match 서브모듈 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/exact.rs`, `cargo test -p axiomme-core --lib`, `bash scripts/quality_gates.sh` |
| TASK-032 | DONE | P0 | user | `crates/axiomme-core/src/index.rs` 대형 모듈 2차 분해(랭킹/스코어링 헬퍼 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/rank.rs`, `cargo check -p axiomme-core --lib`, `cargo test -p axiomme-core --lib` |
| TASK-033 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 1차 분해(types/promotion 모듈 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/types.rs`, `crates/axiomme-core/src/session/commit/promotion.rs`, `cargo check -p axiomme-core --lib` |
| TASK-034 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 2차 분해(dedup/LLM selection 모듈 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/dedup.rs`, `cargo test -p axiomme-core --lib session::commit::tests::`, `cargo test -p axiomme-core --lib` |
| TASK-035 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 3차 분해(write path 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/write_path.rs`, `cargo check -p axiomme-core --lib`, `bash scripts/quality_gates.sh` |
| TASK-036 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 4차 분해(read path 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/read_path.rs`, `cargo test -p axiomme-core --lib session::commit::tests::`, `cargo test -p axiomme-core --lib`, `bash scripts/quality_gates.sh` |
| TASK-037 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 5차 분해(checkpoint/apply orchestration 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/apply_flow.rs`, `cargo test -p axiomme-core --lib session::commit::tests::`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-038 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 6차 분해(promotion apply mode 핸들러 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/apply_modes.rs`, `cargo test -p axiomme-core --lib session::commit::tests::`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-039 | DONE | P1 | user | `crates/axiomme-core/src/session/commit` 7차 분해(memory fallback 기록 경계 분리) | `crates/axiomme-core/src/session/commit/mod.rs`, `crates/axiomme-core/src/session/commit/fallbacks.rs`, `cargo test -p axiomme-core --lib session::commit::tests::`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-040 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 1차 분해(workspace command side-effect 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/workspace_command.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-041 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 2차 분해(episodic semver 파싱/정책 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/episodic_semver.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-042 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 3차 분해(contract probe 실행 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/contract_probe.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-043 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 4차 분해(policy 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/policy.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-044 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 5차 분해(build-quality 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/build_quality.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-045 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 6차 분해(workspace 경계 분리 + 경로 검증 회귀 테스트 추가) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/workspace.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-046 | DONE | P1 | user | `crates/axiomme-core/src/index.rs` 3차 분해(filter 경계 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/filter.rs`, `cargo test -p axiomme-core --lib index::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-047 | DONE | P1 | user | `crates/axiomme-core/src/index.rs` 4차 분해(ancestor/filter projection 경계 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/ancestry.rs`, `cargo test -p axiomme-core --lib index::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-048 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 7차 분해(decision constructor 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/decision.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-049 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 8차 분해(test fixture helper 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/test_support.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-050 | DONE | P1 | user | `crates/axiomme-core/src/index.rs` 5차 분해(lifecycle mutation 경계 분리 + quality gate drift 정합화) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/lifecycle.rs`, `crates/axiomme-core/src/retrieval/expansion.rs`, `cargo test -p axiomme-core --lib index::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-051 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 9차 분해(inline 통합 테스트 파일 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/tests.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-052 | DONE | P1 | user | `crates/axiomme-core/src/index.rs` 6차 분해(search orchestration 경계 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/search_flow.rs`, `cargo test -p axiomme-core --lib index::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-053 | DONE | P1 | user | `crates/axiomme-core/src/release_gate.rs` 10차 분해(contract-integrity orchestration 경계 분리) | `crates/axiomme-core/src/release_gate.rs`, `crates/axiomme-core/src/release_gate/contract_integrity.rs`, `cargo test -p axiomme-core --lib release_gate::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |
| TASK-054 | DONE | P1 | user | `crates/axiomme-core/src/index.rs` 7차 분해(text assembly/helper 경계 분리) | `crates/axiomme-core/src/index.rs`, `crates/axiomme-core/src/index/text_assembly.rs`, `cargo test -p axiomme-core --lib index::tests::`, `cargo test -p axiomme-core --lib`, `cargo check --workspace --all-targets`, `bash scripts/quality_gates.sh` |

## Lifecycle Log
1. `TASK-030` `TODO -> DOING`
2. `TASK-030` `DOING -> DONE`
   - Evidence: canonical docs 재작성, 날짜형 문서 `docs/archive/` 이관, signoff stable 문서/스크립트 경로 정렬.
3. `TASK-031` `TODO -> DOING -> DONE`
   - Evidence: `index.rs` exact-match 블록을 `index/exact.rs`로 분리하고 회귀 테스트 및 quality gates 통과.
4. `TASK-032` `TODO -> DOING -> DONE`
   - Evidence: `index.rs` 랭킹 헬퍼를 `index/rank.rs`로 분리하고 `index.rs` LOC를 1843 -> 1656으로 축소.
5. `TASK-033` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs` LOC를 1611 -> 1211로 축소하고, data model/constants(`types.rs`) 및 promotion helper(`promotion.rs`)를 분리.
6. `TASK-034` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`에서 dedup/LLM selection 블록을 `dedup.rs`로 분리하고 LOC를 1211 -> 941로 축소.
7. `TASK-035` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`의 write side-effect 경계를 `write_path.rs`로 분리하고 LOC를 941 -> 822로 축소.
8. `TASK-036` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`의 read/list 경계를 `read_path.rs`로 분리하고 LOC를 822 -> 739로 축소.
9. `TASK-037` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`의 checkpoint/apply state-machine 오케스트레이션을 `apply_flow.rs`로 분리하고 LOC를 739 -> 612로 축소.
10. `TASK-038` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`의 apply mode 핸들러(`all_or_nothing`/`best_effort`)를 `apply_modes.rs`로 분리하고 LOC를 612 -> 512로 축소.
11. `TASK-039` `TODO -> DOING -> DONE`
   - Evidence: `session/commit/mod.rs`의 fallback 기록 함수(`record_memory_extractor_fallback`, `record_memory_dedup_fallback`)를 `fallbacks.rs`로 분리하고 LOC를 512 -> 491로 축소.
12. `TASK-040` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 workspace command 실행/테스트 모킹 경계를 `release_gate/workspace_command.rs`로 분리하고 회귀 테스트(`release_gate::tests::`) 및 quality gates를 재통과.
13. `TASK-041` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 episodic semver 파싱/판정 로직을 `release_gate/episodic_semver.rs`로 분리하고 LOC를 1471 -> 1245로 축소, 전체 게이트 재통과.
14. `TASK-042` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 contract probe 실행 경계(`run_contract_execution_probe`, `run_episodic_api_probe`, `run_ontology_contract_probe`)를 `release_gate/contract_probe.rs`로 분리하고 LOC를 1245 -> 1112로 축소, 전체 게이트 재통과.
15. `TASK-043` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 policy 경계(`episodic_semver_policy`, `ontology_contract_policy`)를 `release_gate/policy.rs`로 분리하고 LOC를 1112 -> 1101로 축소, 전체 게이트 재통과.
16. `TASK-044` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 build-quality 경계(`evaluate_build_quality_gate`)를 `release_gate/build_quality.rs`로 분리하고 LOC를 1101 -> 1081로 축소, 전체 게이트 재통과.
17. `TASK-045` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 workspace 경계(`resolve_workspace_dir`)를 `release_gate/workspace.rs`로 분리하고 경로 검증 회귀 테스트 3건(`resolve_workspace_dir_*`)을 추가, 전체 게이트 재통과.
18. `TASK-046` `TODO -> DOING -> DONE`
   - Evidence: `index.rs`의 filter 경계(`normalize_filter`, `leaf_matches_filter`, `record_matches_filter`)를 `index/filter.rs`로 분리하고 LOC를 1656 -> 1602로 축소, 전체 게이트 재통과.
19. `TASK-047` `TODO -> DOING -> DONE`
   - Evidence: `index.rs`의 ancestor/filter projection 경계(`has_matching_leaf_descendant`, `filter_projection_uris`)를 `index/ancestry.rs`로 분리하고 LOC를 1602 -> 1561로 축소, 전체 게이트 재통과.
20. `TASK-048` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 decision constructor 경계(`gate_decision`, `*_gate_decision`, `finalize_release_gate_pack_report`)를 `release_gate/decision.rs`로 분리하고 LOC를 1105 -> 993으로 축소, 전체 게이트 재통과.
21. `TASK-049` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs` 테스트 픽스처 헬퍼(`eval_report`, `benchmark_gate_result`, `write_contract_gate_workspace_fixture`)를 `release_gate/test_support.rs`로 분리하고 LOC를 993 -> 878로 축소, 전체 게이트 재통과.
22. `TASK-050` `TODO -> DOING -> DONE`
   - Evidence: `index.rs`의 child-index/lexical lifecycle mutation 경계(`remove_lexical_stats`, `upsert_child_index_entry`, `remove_child_index_entry`)를 `index/lifecycle.rs`로 분리하고 LOC를 1561 -> 1515로 축소, 검증 중 발생한 clippy drift(`manual_is_multiple_of`)를 `retrieval/expansion.rs` 한 줄 정합화로 해소한 뒤 전체 게이트 재통과.
23. `TASK-051` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 inline 통합 테스트 모듈(`mod tests`)을 `release_gate/tests.rs`로 분리하고 production 파일 LOC를 878 -> 269로 축소, 분리 직후 formatting drift(파일 선행 공백 1라인)를 정리한 뒤 전체 게이트 재통과.
24. `TASK-052` `TODO -> DOING -> DONE`
   - Evidence: `index.rs`의 검색 오케스트레이션 경계(`search`, `search_directories`)를 `index/search_flow.rs`로 분리하고 `index.rs` LOC를 1515 -> 1391로 축소, 분리 후 전체 게이트 재통과.
25. `TASK-053` `TODO -> DOING -> DONE`
   - Evidence: `release_gate.rs`의 contract-integrity 오케스트레이션 경계(`evaluate_contract_integrity_gate`)를 `release_gate/contract_integrity.rs`로 분리하고 `release_gate.rs` LOC를 269 -> 245로 축소, 분리 후 전체 게이트 재통과.
26. `TASK-054` `TODO -> DOING -> DONE`
   - Evidence: `index.rs`의 text assembly/helper 경계(`build_upsert_text`)를 `index/text_assembly.rs`로 분리하고 `index.rs` LOC를 1391 -> 1367로 축소, 분리 후 전체 게이트 재통과.

## Next Action Mapping
- Selected For Next: NONE
