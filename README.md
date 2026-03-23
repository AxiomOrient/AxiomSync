# AxiomSync

Universal agent memory kernel for recording immutable raw records into a single SQLite `context.db`, projecting them into canonical views, and deriving evidence-backed knowledge over CLI, HTTP, MCP, and a Rust-rendered web UI.

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
cargo run -p axiomsync -- project plan-rebuild
cargo run -p axiomsync -- project apply-replay-plan --file /tmp/replay-plan.json
cargo run -p axiomsync -- derive plan
cargo run -p axiomsync -- derive apply-plan --file /tmp/derive-plan.json
cargo run -p axiomsync -- search "timeout error"
cargo run -p axiomsync -- project plan-auth-grant --workspace-root /repo/app --token secret-token
cargo run -p axiomsync -- project plan-admin-grant --token admin-secret-token
cargo run -p axiomsync -- web --addr 127.0.0.1:4400
```

## Canonical Sink Flow
- `sink plan-append-raw-events`: build `IngestPlan` from immutable request data
- `sink apply-ingest-plan`: apply a previously serialized `IngestPlan`
- `sink plan-upsert-source-cursor`: build `SourceCursorUpsertPlan` without mutating the store
- `sink apply-source-cursor-plan`: apply a previously serialized cursor plan
- `web`: serves `GET /health`, query/admin routes, and `/sink/*` on one server

## Kernel Flow
- `project plan-rebuild`: produce `ReplayPlan` from current raw ledger plus collected enrichment
- `project apply-replay-plan`: apply a previously serialized `ReplayPlan`
- `derive plan`: collect derivation enrichment and produce `DerivePlan`
- `derive apply-plan`: apply a previously serialized `DerivePlan`
- `mcp serve`: expose canonical `search_cases`, `get_case`, `get_thread`, `get_evidence`, `list_runs`, `get_run`, `list_documents`, `get_document`

## Canonical Query Model
- canonical nouns: `case`, `thread`, `run`, `task`, `document`, `evidence`
- legacy aliases remain available for compatibility: `episode`, `runbook`
- record ingestion accepts multiple producers such as `codex`, `claude_code`, `gemini_cli`, and internal runtimes

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
cargo run -p axiomsync -- web --help
cargo run -p axiomsync -- mcp serve --help
```
