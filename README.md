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
- Canonical query helpers: `search_insights`, `find_fix`, `find_decision`, `find_runbook`, `get_evidence_bundle`
- Unified retrieval helper: `search_docs`

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
cargo run -p axiomsync -- project rebuild
cargo run -p axiomsync -- project doctor
cargo run -p axiomsync -- query search-docs --file /tmp/search-docs.json
cargo run -p axiomsync -- query search-insights --file /tmp/search-insights.json
cargo run -p axiomsync -- query find-fix --file /tmp/find-fix.json
cargo run -p axiomsync -- query get-evidence-bundle --subject-kind insight --subject-id insight_123
cargo run -p axiomsync -- project plan-auth-grant --workspace-root /repo/app --token secret-token
cargo run -p axiomsync -- project plan-admin-grant --token admin-secret-token
cargo run -p axiomsync -- serve --addr 127.0.0.1:4400
```

## Canonical Sink Flow
- `sink plan-append-raw-events`: build `IngestPlan` from immutable request data
- `sink apply-ingest-plan`: apply a previously serialized `IngestPlan`
- `sink plan-upsert-source-cursor`: build `SourceCursorUpsertPlan` without mutating the store
- `sink apply-source-cursor-plan`: apply a previously serialized cursor plan
- sink input accepts both the legacy flat event shape and the final-form envelope shape with root `source.{source_kind,connector_name}` plus `events[]`
- `serve`: serves `GET /health`, query/admin routes, and `/sink/*` on one server
- external collectors and edge runtimes integrate only through these `sink` routes or equivalent CLI commands

## Kernel Flow
- `project rebuild`: replay projection and derivation from the raw ledger
- `project doctor`: report counts for receipts, projected rows, derived rows, and pending projection/derivation/index work
- `mcp serve`: expose canonical session/episode/insight/procedure resources plus compatibility aliases

## Canonical Query Model
- canonical nouns: `session`, `entry`, `artifact`, `anchor`, `episode`, `insight`, `procedure`
- compatibility aliases remain available: `claim`, `case`, `thread`, `runbook`, `task`, `document`, `evidence`
- derived knowledge is centered on `episodes + insights + verifications + procedures`; `claims` remains a compatibility read model
- record ingestion accepts multiple producers such as `codex`, `claude_code`, `gemini_cli`, AxiomRelay, and AxiomRams

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
