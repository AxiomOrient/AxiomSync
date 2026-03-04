# OM Boundary Accuracy Plan (2026-03-04)

## Goal
- Close remaining OM boundary findings with correctness-first behavior.
- Keep episodic (pure contract/transform) and AxiomMe (runtime/persistence) boundaries explicit.

## Scope
- release gate contract probe hardening for prompt-contract version/signature policy.
- observer/reflector fallback parser contract strictness.
- continuation reducer source-of-truth persistence alignment.
- buffered reflector selection fallback simplification (entry-first only).

## Data Model
- Continuation persistence stores reducer-resolved `source_kind` as canonical storage string.
- Observer/reflector fallback acceptance requires explicit contract marker in content path.
- Buffered reflector selection no longer depends on legacy line-slice raw active observation fallback.

## Transformations vs Side Effects
- Pure: contract marker checks, source-kind mapping, reflector entry selection calculations.
- Side effects: sqlite upserts and release-gate command probes remain isolated in existing boundary layers.

## Concurrency Notes
- No new shared mutable state introduced.
- Existing observer batch threading behavior unchanged.

## Verification Map
1. Narrow tests
   - observer parser fallback contract tests
   - reflector parser fallback contract tests
   - continuation upsert source-kind tests
   - release probe contract signature test
2. Broader tests
   - `cargo test -p axiomme-core --quiet`
   - `cargo clippy -p axiomme-core --all-targets -- -D warnings`
   - `cargo audit -q`

## Execution Validation (2026-03-04)
- Applied
  - release probe now includes prompt-contract signature lock keyed by `contract_version/protocol_version`.
  - observer/reflector fallback parser paths require contract marker in content.
  - continuation persistence stores reducer-resolved canonical `source_kind`.
  - buffered reflector selector removed raw line-slice fallback when active entries are empty/blank.
- Verified
  - `cargo test -p axiomme-core --quiet`
  - `cargo clippy -p axiomme-core --all-targets -- -D warnings`
  - `cargo audit -q`

## Execution Validation (2026-03-05)
- Applied
  - fixed release-pack trace metrics test command mocks to match `contract_probe` git policy path (`git show HEAD~1:...` and `git show HEAD:...`).
  - hardened episodic user-prompt boundary handling so multi-thread existing observations and reflector user-provided blocks are consistently escaped as data blocks.
  - strengthened episodic OM invariant checks with explicit RFC3339 validation for `created_at_rfc3339` and `materialized_at_rfc3339`.
- Verified
  - `cargo test -p axiomme-core --lib client::tests::release_contract_pack_tracemetrics::release_gate_pack_orchestrates_decisions_with_mocked_workspace_commands -- --exact`
  - `cargo test -p axiomme-core --lib`
  - `cargo test -p episodic prompt::tests::`
  - `cargo test -p episodic model::tests::`
  - `cargo test -p episodic --lib`
