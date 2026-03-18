# AxiomSync

Local-first context runtime and operator CLI for agentic systems.

AxiomSync는 `axiom://` URI, `context.db`, 메모리 검색 런타임, 세션/OM 상태를 하나로 묶는 로컬 런타임입니다. 이 저장소는 런타임과 CLI만 소유합니다.

## Release Line
- Current repository release line: `v1.3.0`
- Canonical local store: `<root>/context.db`
- Retrieval policy: `memory_only`
- Persistence policy: SQLite only

## Repository Boundary
- In this repository: `crates/axiomsync`, `docs/`, `scripts/`
- Outside this repository: web companion, mobile FFI companion, app-specific frontend shells

## Quick Start
```bash
cargo run -p axiomsync -- --help

cargo run -p axiomsync -- init
cargo run -p axiomsync -- add ./docs --target axiom://resources/docs
cargo run -p axiomsync -- search "oauth flow"
cargo run -p axiomsync -- session commit
```

## Quick Scenario Checks

실사용 점검을 위한 단일 진입점:

```bash
bash scripts/run_quick_scenario_checks.sh \
  --iterations 5 \
  --seed 20260318 \
  --timeout 90 \
  --scenario random \
  --max-cold-ms 1200 \
  --max-p95-ms 700 \
  --min-queue-eps 50 \
  --summary-format json \
  --summary-out /tmp/axiomsync-quick-run-summary.json
```

- `--summary-format text`: 텍스트 요약 저장(기본값)
- `--summary-format json`: JSON 요약 저장
- `RESULT_WARNING counts_match=false`: 집계 불일치 경고(로그/이력 재점검 필요)

## Runtime Model
- URI model: `axiom://{scope}/{path}`
- State store: `context.db`
- Retrieval runtime: `memory_only`
- Persisted retrieval state: `search_docs` + `search_docs_fts`
- Canonical result shape: `FindResult.query_results` + `hit_buckets`
- Compatibility JSON views: serialized `memories`, `resources`, `skills`
- Derived bucket views: `FindResult.memories()`, `resources()`, `skills()`
- Session/OM state: explicit and durable

## Documentation Map
- [docs/INDEX.md](./docs/INDEX.md): documentation entrypoint
- [docs/BLUEPRINT.md](./docs/BLUEPRINT.md): product target state
- [docs/IMPLEMENTATION_SPEC.md](./docs/IMPLEMENTATION_SPEC.md): implementation completion contract
- [docs/API_CONTRACT.md](./docs/API_CONTRACT.md): stable contract
- [docs/RUNTIME_ARCHITECTURE.md](./docs/RUNTIME_ARCHITECTURE.md): runtime structure
- [docs/RETRIEVAL_ARCHITECTURE.md](./docs/RETRIEVAL_ARCHITECTURE.md): retrieval path
- [docs/RELEASE_RUNBOOK.md](./docs/RELEASE_RUNBOOK.md): release owner checklist
- [docs/CODE_OWNERSHIP.md](./docs/CODE_OWNERSHIP.md): change routing
- [docs/USER_SCENARIO_TEST_PLAYBOOK.md](./docs/USER_SCENARIO_TEST_PLAYBOOK.md): scenario prompts and user-facing test workflows
- [scripts/run_quick_scenario_checks.sh](./scripts/run_quick_scenario_checks.sh): scenario check runner (bash 실행 권장)

## Quality And Release
```bash
bash scripts/quality_gates.sh
bash scripts/release_pack_strict_gate.sh --workspace-dir "$(pwd)"
```

## Non-Negotiable Rules
- Canonical URI protocol stays `axiom://`
- Runtime startup is a hard cutover to `context.db`
- Legacy DB filename discovery or migration is not supported
- Retrieval backend remains `memory_only`; `sqlite` retrieval mode is rejected
- Vendored pure-OM boundary remains explicit under `axiomsync::om`
