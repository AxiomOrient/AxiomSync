# Feature Completeness / UAT Gate

## Scope
- runtime contract completeness
- manual use-case coverage
- release signoff readiness

## Functional Coverage
| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| FR-001 | URI/scope parsing and restrictions | PASS | `axiomme-core` retrieval/fs tests + CLI smoke |
| FR-002 | Tiered context (`.abstract.md`, `.overview.md`) | PASS | core fs/client tests |
| FR-003 | Resource ingest + replay-safe updates | PASS | queue/reconcile lifecycle tests |
| FR-004 | Deterministic retrieval + trace | PASS | retrieval/search tests |
| FR-005 | Session/memory lifecycle | PASS | session tests |
| FR-006 | Package import/export safety | PASS | release contract pack tests |
| FR-007 | Observability/evidence artifacts | PASS | release/trace evidence commands |
| FR-008 | Naming migration (`axiom://`) | PASS | prohibited token scan + docs sync |
| FR-009 | Replacement validation | PASS | contract probe tests |
| FR-010 | Embedding reliability/gates | PASS | benchmark/eval gates |
| FR-011 | Web handoff contract | PASS | `axiomme web` handoff + API probe |

## UAT Scenario Coverage
| Scenario | Status |
| --- | --- |
| Resource Lifecycle | PASS |
| Traceable Retrieval | PASS |
| Session Memory Evolution | PASS |
| Package Safety | PASS |
| Internal Scope Governance | PASS |

## Signoff
- Platform/Tooling: `DONE`
- Final Release Decision: `DONE (aiden, GO)`

## Deterministic Re-check
```bash
bash scripts/quality_gates.sh
bash scripts/release_pack_strict_gate.sh --workspace-dir . --output logs/release_pack_strict_report.json
scripts/release_signoff_status.sh --report-path docs/RELEASE_SIGNOFF_STATUS.md
```
