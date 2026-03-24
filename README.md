# AxiomSync

Universal agent memory kernel for recording immutable raw records into a single SQLite `context.db`, projecting them into canonical views, and deriving evidence-backed knowledge over CLI, HTTP, MCP, and a Rust-rendered web UI.

## Repository Boundary
- This repository owns the kernel workspace, CLI, HTTP API, MCP surface, and local web UI.
- The canonical external write boundary is the raw-only `sink` surface exposed by this repository.
- Edge capture, spool, retry, approval, browser integration, and connector-specific delivery live in a separate external repository and are not part of this release surface.
- Any review package or extraction notes checked into this repository are reference material only, not the release contract.

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
- Sink routes are intentionally unauthenticated but loopback-only, even if `web` is bound to a non-loopback address.

## Quick Start
```bash
cargo run -p axiomsync -- --help

cargo run -p axiomsync -- init
cargo run -p axiomsync -- sink plan-append-raw-events --file /tmp/raw-events.json
cargo run -p axiomsync -- sink apply-ingest-plan --file /tmp/ingest-plan.json
cargo run -p axiomsync -- sink plan-upsert-source-cursor --file /tmp/cursor.json
cargo run -p axiomsync -- sink apply-source-cursor-plan --file /tmp/cursor-plan.json
cargo run -p axiomsync -- project plan-rebuild > /tmp/replay-plan.json
cargo run -p axiomsync -- project apply-replay-plan --file /tmp/replay-plan.json
cargo run -p axiomsync -- project doctor
cargo run -p axiomsync -- sink import-cli-run --file /tmp/cli-run.json
cargo run -p axiomsync -- sink import-work-state --file /tmp/work-state.json
cargo run -p axiomsync -- query search-cases --file /tmp/search-cases.json
cargo run -p axiomsync -- query get-case --id case_123
cargo run -p axiomsync -- query get-thread --id thread_123
cargo run -p axiomsync -- query get-run --id run_123
cargo run -p axiomsync -- project plan-auth-grant --workspace-root /repo/app --token secret-token
cargo run -p axiomsync -- project plan-admin-grant --token admin-secret-token
cargo run -p axiomsync -- serve --addr 127.0.0.1:4400
```

## Canonical Sink Flow
- `sink plan-append-raw-events`: build `IngestPlan` from immutable request data
- `sink apply-ingest-plan`: apply a previously serialized `IngestPlan`
- `sink plan-upsert-source-cursor`: build `SourceCursorUpsertPlan` without mutating the store
- `sink apply-source-cursor-plan`: apply a previously serialized cursor plan
- sink append request shape is `AppendRawEventsRequest { batch_id, producer, received_at_ms, events[] }`
- source cursor request shape is `UpsertSourceCursorRequest { connector, cursor_key, cursor_value, updated_at_ms }`
- `serve`: serves `GET /health`, query/admin routes, and `/sink/*` on one server
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
- Architecture: [`docs/RUNTIME_ARCHITECTURE.md`](./docs/RUNTIME_ARCHITECTURE.md)
- Testing: [`docs/TESTING.md`](./docs/TESTING.md)
- Release checklist: [`docs/RELEASE_RUNBOOK.md`](./docs/RELEASE_RUNBOOK.md)

## Verification
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- sink --help
cargo run -p axiomsync -- serve --help
cargo run -p axiomsync -- mcp serve --help
```
