# Release Hardening Plan

## Mission
Close the remaining correctness and release-trust blockers that still prevent external or production release of AxiomSync. This plan replaces the completed application-service refactor plan with a release-hardening plan whose only goal is to make the current single-crate runtime safe to ship.

## Why This Plan Exists
The large structural cleanup is mostly complete: the workspace boundary is now `crates/axiomsync`, the application-service split is in place, and the runtime/doctor/migrate/release surfaces are materially cleaner. What remains is not cosmetic follow-up. The remaining issues are correctness, contract-enforcement, and release-gate fidelity defects that can still produce wrong behavior or misleading operator confidence.

This plan treats the following as hard blockers for external release:
1. `mount_repo` still derives `content_hash` from the `source_path` string instead of a stable repository tree digest.
2. `LinkService` does not enforce the published `relation` identifier contract.
3. `ArchiveService` still uses unnecessary `unsafe String::from_utf8_unchecked`.
4. `scripts/release_pack_strict_gate.sh` is not actually strict because it defaults to a debug binary and permissive benchmark thresholds.
5. `mount_repo` root-resource search projection can still be overwritten by global reindex, and the docs still carry this as a known behavioral note.

## Planning Goal
Deliver a bounded hardening pass that removes the five remaining blockers, aligns code/tests/docs/release scripts to the same contract, and produces evidence strong enough to re-evaluate external release readiness.

## Scope

### In Scope
- Repository-mount identity and hash semantics.
- Repository-mount root resource projection stability during global reindex.
- Link relation input contract enforcement.
- Archive JSONL assembly safety and behavior preservation.
- Strict release-pack shell gate fidelity and parity with CLI defaults.
- Tests, docs, and release-readiness wording directly affected by the five blockers.

### Primary Code Paths
- `crates/axiomsync/src/client/repo.rs`
- `crates/axiomsync/src/client/link.rs`
- `crates/axiomsync/src/client/archive.rs`
- `crates/axiomsync/src/client/resource.rs`
- `crates/axiomsync/src/client/runtime.rs`
- `crates/axiomsync/src/client/release/pack_service.rs`
- `crates/axiomsync/src/cli/release.rs`
- `scripts/release_pack_strict_gate.sh`
- `crates/axiomsync/src/client/tests/facade.rs`
- `tests/process_contract.rs`
- `tests/repository_markdown_user_flows.rs`
- affected docs under `docs/` and `plans/`

### Out of Scope
- New features, new companion services, or frontend expansion.
- Another broad application-service refactor cycle.
- Unrelated performance cleanup not required to close one of the five blockers.
- General backlog work that does not change release readiness.

## Delivery Contract
This plan is complete only when all of the following are true:
1. Repository mount identity uses a stable digest of repository content and metadata relevant to mount correctness rather than the raw path string.
2. The global reindex path no longer destroys or mutates mount root projection fields that must remain mount-specific.
3. Link creation rejects invalid `relation` identifiers at the service boundary according to the published contract.
4. Archive JSONL assembly no longer relies on unnecessary `unsafe`, and behavior remains covered by regression tests.
5. The strict shell release gate uses a release-grade binary path and strict defaults that are aligned with the CLI/operator contract rather than a weaker parallel source of truth.
6. Tests and docs are updated to validate the fixed behavior instead of encoding the old defects.
7. Final hardening verification can explicitly state whether external release is now unblocked by these five issues.

## Constraints
- Prefer the shortest path to externally verifiable correctness.
- Every blocker fix must update the nearest regression test or add one if the defect currently lacks protection.
- Do not leave “known issue” wording in docs for behavior that is supposed to be closed by this plan.
- Release-gate defaults must have one clear contract source and shell/CLI parity.
- Do not expand scope into unrelated cleanup unless required to close a blocker or make verification possible.

## Critical Path
`RH-00 Freeze release-hardening contract` -> `RH-01 Fix repository tree digest semantics` -> `RH-02 Stabilize mount root projection under global reindex` -> `RH-05 Make strict release gate actually strict` -> `RH-06 Refresh docs and release-readiness wording` -> `RH-07 Run focused regression matrix` -> `RH-08 Run final release-hardening gate`

Rationale:
- `RH-01` and `RH-02` sit on the primary correctness path because repository mount identity and mount projection stability directly affect persisted truth and search behavior.
- `RH-05` sits on the critical path because release judgement must not rely on a misleading “strict” gate.
- `RH-03` and `RH-04` are mandatory but parallelizable hardening tracks because they do not block the repo-mount fix sequence.

## Execution Phases

### Phase 0. Freeze the Release-Hardening Contract
Objective:
- Replace the completed refactor plan with a hardening-specific plan and task ledger.

Outputs:
- New `plans/IMPLEMENTATION-PLAN.md`
- New `plans/TASKS.md`

Verification:
- Both files describe the same blocker set, decision gates, and done condition.

### Phase 1. Fix Repository Mount Identity
Objective:
- Replace path-string hashing with a stable repository tree digest that reflects mounted content identity.

Required outcomes:
- Hash input model is defined and deterministic.
- Tests stop asserting the broken path-string behavior.
- Any migration or compatibility impact is made explicit in code/tests/docs.

Verification:
- Focused tests prove the digest changes when repository content changes and remains stable for identical content.

### Phase 2. Stabilize Mount Root Projection Through Reindex
Objective:
- Ensure global reindex cannot overwrite mount-root projection fields that must survive as mount-specific metadata.

Required outcomes:
- Reindex logic distinguishes mount-root records from ordinary resource projection updates.
- The old known-note behavior is reproduced in a regression test before or during the fix, then removed from docs after the fix.

Verification:
- A focused integration path covers `mount_repo` followed by global reindex and confirms projection invariants remain intact.

### Phase 3. Harden Service Contracts
Objective:
- Enforce link `relation` validity at the service boundary and remove unnecessary archive `unsafe`.

Required outcomes:
- Invalid relation inputs fail fast with clear error behavior.
- Valid relation inputs continue to work.
- Archive JSONL assembly uses safe UTF-8 construction without changing output semantics.

Verification:
- Targeted service-level tests cover valid/invalid relations and archive output parity.

### Phase 4. Align the Strict Release Gate
Objective:
- Make the shell strict gate reflect the actual operator contract.

Required outcomes:
- The script defaults to a release-grade binary path.
- Benchmark defaults are strict enough to match the release CLI contract or explicitly share the same source of truth.
- The naming and behavior of “strict” are no longer misleading.

Verification:
- Script invocation and CLI defaults are compared directly.
- Any documented strict thresholds match the executable defaults.

### Phase 5. Refresh Docs and Release Readiness
Objective:
- Remove obsolete known-issue wording and rewrite release-readiness language around the hardened state only after fixes land.

Required outcomes:
- Docs no longer imply the old blocker behavior is acceptable.
- Release-readiness wording clearly distinguishes internal dogfood suitability from external release criteria when needed.

Verification:
- A docs sweep confirms the old known-note and stale acceptance language are removed or rewritten.

### Phase 6. Final Evidence Pass
Objective:
- Gather the minimum strong evidence needed to decide whether these five blockers are truly closed.

Required outcomes:
- Focused regression tests pass.
- Repository-wide quality gates relevant to release readiness pass.
- The task ledger and docs reflect the same final state.

Verification:
- Final gate commands and focused tests are recorded as evidence under the task ledger.

## Decision Gates

| Gate | Check | Passes When | On Fail |
|---|---|---|---|
| Repo Identity Gate | `mount_repo` hash semantics | mounted repo identity is derived from stable tree content semantics, not raw path text | stop release-hardening execution, redesign digest input model, update tests first |
| Projection Stability Gate | mount root survives global reindex | reindex preserves mount-specific root projection invariants | keep docs in blocked state, add/repair regression test, do not advance release wording |
| Contract Safety Gate | link/archive hardening | invalid relations are rejected and archive flow is safe without `unsafe` regression | block doc refresh and final gate until service boundary behavior is proven |
| Strict Gate Alignment | shell/CLI parity | strict shell gate runs release-grade path and strict defaults are aligned with CLI/operator contract | treat release evidence as unreliable and block external-release judgement |
| External Release Readiness Gate | final evidence review | all five blockers are closed in code, tests, docs, and release scripts with explicit evidence | mark plan incomplete and keep external release blocked |

## Verification Strategy

### Focused Regression Checks
- Repository mount digest tests:
  - stable for identical content
  - changes for content mutation
  - does not use raw path-string hash semantics
- Mount root projection tests:
  - `mount_repo` then global `reindex_all`
  - root `search_docs.namespace/kind` invariants preserved
- Link contract tests:
  - accept `[a-z0-9_-]`
  - reject invalid characters and malformed variants
- Archive safety tests:
  - archive plan/execute output stays valid
  - no behavior drift after safe UTF-8 conversion
- Strict release gate tests:
  - shell defaults match intended strict contract
  - release binary path and benchmark thresholds are explicit and consistent

### Final Quality Gates
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p axiomsync`
- `cargo audit --deny unsound --deny unmaintained --deny yanked`
- any smallest focused test commands added for the blocker-specific fixes

## Risk Notes
- Repository digest semantics can affect existing expectations, fixtures, or persisted records; treat compatibility explicitly rather than letting it drift.
- Mount projection correctness is easy to “fix” partially while leaving a secondary overwrite path alive; the regression test must exercise the exact mount-plus-reindex path.
- Shell/CLI contract drift can reappear if defaults remain duplicated without a clear source of truth.

## Exit Condition
The plan ends only when the five named blockers are closed with code/test/doc/script evidence and the repository can be re-assessed for external release without carrying forward any of those blockers as accepted known issues.
