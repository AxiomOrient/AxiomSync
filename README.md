# AxiomSync

Universal agent memory kernel that records immutable raw events into one SQLite `context.db`, projects canonical views, and derives evidence-backed knowledge over CLI, HTTP, MCP, and a local web UI.

## Repository Boundary
- This repository owns the kernel workspace, CLI, HTTP API, MCP surface, and local web UI.
- The canonical external write boundary is the raw-only `sink` surface exposed by this repository.
- Edge capture, spool, retry, approval, browser integration, and connector-specific delivery live in a separate external repository and are not part of this release surface.
- Only files explicitly linked from this README, `docs/`, or `scripts/verify-release.sh` are part of the release contract.

## Runtime Model
- Domain state: single SQLite store at `<root>/context.db`
- Auth grants: `<root>/auth.json`
- Core pipeline: `Parse -> Normalize -> Plan -> Apply`
- Core model: `record -> view -> knowledge`
- Determinism: IDs and hashes are derived from canonicalized input JSON
- Canonical write boundary: raw-only `sink` surface
- Public surfaces: CLI, HTTP API, MCP (`stdio` + HTTP), Maud web UI
- Canonical HTTP base: `http://127.0.0.1:4400`
- Canonical sink paths: `/sink/raw-events/plan`, `/sink/raw-events/apply`, `/sink/source-cursors/plan`, `/sink/source-cursors/apply`
- Canonical query helpers: `search_cases`, `get_case`, `get_thread`, `get_run`, `get_task`, `get_document`, `get_evidence`
- Local trusted imports: `import-cli-run`, `import-work-state`

## Security Notes
- `auth.json` stores hashed workspace grants plus hashed global admin tokens and is written with owner-only permissions on Unix.
- Workspace-scoped read routes require a workspace bearer token.
- Admin HTTP and web routes require a global admin bearer token.
- Sink routes are intentionally unauthenticated but loopback-only, even if `serve` is bound to a non-loopback address.
- Same-host relay adapter sequencing is fixed in `docs/RELAY_INTEROP.md`.

## Quick Start
```bash
cargo run -p axiomsync-cli -- --help

cargo run -p axiomsync-cli -- init
cargo run -p axiomsync-cli -- sink plan-append-raw-events --file /tmp/raw-events.json
cargo run -p axiomsync-cli -- sink apply-ingest-plan --file /tmp/ingest-plan.json
cargo run -p axiomsync-cli -- sink plan-upsert-source-cursor --file /tmp/cursor.json
cargo run -p axiomsync-cli -- sink apply-source-cursor-plan --file /tmp/cursor-plan.json
cargo run -p axiomsync-cli -- project plan-rebuild > /tmp/replay-plan.json
cargo run -p axiomsync-cli -- project apply-replay-plan --file /tmp/replay-plan.json
cargo run -p axiomsync-cli -- project doctor
cargo run -p axiomsync-cli -- sink import-cli-run --file /tmp/cli-run.json
cargo run -p axiomsync-cli -- sink import-work-state --file /tmp/work-state.json
cargo run -p axiomsync-cli -- query search-cases --file /tmp/search-cases.json
cargo run -p axiomsync-cli -- query get-case --id case_123
cargo run -p axiomsync-cli -- query get-thread --id thread_123
cargo run -p axiomsync-cli -- query get-run --id run_123
cargo run -p axiomsync-cli -- project plan-auth-grant --workspace-root /repo/app --token secret-token
cargo run -p axiomsync-cli -- project plan-admin-grant --token admin-secret-token
cargo run -p axiomsync-cli -- serve --addr 127.0.0.1:4400
```

## Canonical Sink Flow
- `sink plan-append-raw-events`: build `IngestPlan` from immutable request data
- `sink apply-ingest-plan`: apply a previously serialized `IngestPlan`
- `sink plan-upsert-source-cursor`: build `SourceCursorUpsertPlan` without mutating the store
- `sink apply-source-cursor-plan`: apply a previously serialized cursor plan
- sink append request shape is `AppendRawEventsRequest { batch_id, producer, received_at_ms, events[] }`
- source cursor request shape is `UpsertSourceCursorRequest { connector, cursor_key, cursor_value, updated_at_ms }`
- `serve`: serves `GET /health`, query/admin routes, and `/sink/*` on one server
- collection reads that enumerate workspace data require an explicit workspace selector
- external collectors and edge runtimes integrate only through these `sink` routes or equivalent CLI commands

## Kernel Flow
- `project plan-projection` / `project apply-projection-plan`: rebuild canonical projection through an explicit plan
- `project plan-derivations` / `project apply-derivation-plan`: rebuild derived rows through an explicit plan
- `project plan-rebuild` / `project apply-replay-plan`: rebuild projection and derivation from the raw ledger through one replay plan
- `project doctor`: report counts for receipts, projected rows, derived rows, and pending projection/derivation/index work
- `mcp serve`: expose canonical `case/thread/run/task/document/evidence` resources and tools

## Canonical Query Model
- public canonical nouns: `case`, `thread`, `run`, `task`, `document`, `evidence`
- internal projection/derivation nouns such as `session`, `entry`, `artifact`, `anchor`, `episode`, `insight`, `procedure` remain implementation detail
- record ingestion accepts multiple producers such as `axiomrelay`, `axiomrams`, local CLI importers, and other edge runtimes that target the sink seam

## Release Docs
- Runtime/API: [`docs/API_CONTRACT.md`](./docs/API_CONTRACT.md)
- Sink contract: [`docs/KERNEL_SINK_CONTRACT.md`](./docs/KERNEL_SINK_CONTRACT.md)
- Relay interop: [`docs/RELAY_INTEROP.md`](./docs/RELAY_INTEROP.md)
- Architecture: [`docs/RUNTIME_ARCHITECTURE.md`](./docs/RUNTIME_ARCHITECTURE.md)
- Verification script: [`scripts/verify-release.sh`](./scripts/verify-release.sh)
- Real-user QA package: [`qa/README.md`](./qa/README.md)

`docs/` defines the release contract surface. `scripts/verify-release.sh` is the canonical verification entrypoint.

## Verification
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture
cargo run -p axiomsync-cli -- --help
cargo run -p axiomsync-cli -- sink --help
cargo run -p axiomsync-cli -- serve --help
cargo run -p axiomsync-cli -- mcp serve --help
./scripts/verify-release.sh
```

Primary regression suites:
- `crates/axiomsync-cli/tests/replay_pipeline.rs`
- `crates/axiomsync-cli/tests/sink_contract.rs`
- `crates/axiomsync-cli/tests/http_and_mcp.rs`
- `crates/axiomsync-cli/tests/relay_interop.rs`
- `crates/axiomsync-cli/tests/public_surface_guard.rs`

Deeper user-journey validation:
- `qa/bin/run-real-user-qa.sh`
