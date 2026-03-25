# Changelog

## Unreleased

- added a real-user QA package under `qa/` with scripted CLI, HTTP, MCP, and relay scenarios
- aligned MCP behavior around JSON-RPC `initialize`, structured error responses, and workspace-selector failures
- tightened ingest/replay validation and store query paths to avoid timestamp overflow and full-table dedupe scans

## v1.4.0 - 2026-03-25

- tightened canonical HTTP and MCP workspace/admin scope enforcement and parity coverage
- expanded replay, sink, and public-surface regression coverage around idempotency, ranking, and release guards
- reduced repository docs to the core contract set and made `scripts/verify-release.sh` the canonical verification entrypoint

## v1.3.1 - 2026-03-18

- aligned the standalone spec package, runtime docs, and execution plans around the current `serve`/`mcp serve` contract
- documented sink schema compatibility fields (`native_entry_id`, optional `artifacts`, optional `hints`) without changing the canonical request shape
- tightened release planning and contract-audit artifacts under `plans/` to match the current repository-owned surface

## v1.3.0 - 2026-03-16

- finalized release readiness gates with passing quality and strict release pack validation
- promoted archive, event, link, and repo flows behind thin `facade` delegates and removed `facade_v3`
- added canonical operator commands for `doctor`, `migrate`, and `release verify`
- completed retrieval trace evidence coverage for `mixed_intent`, `restore_source`, and `fts_fallback_used`
- added clean-root process contracts and repository markdown user-flow regression coverage
- reduced script and document surface to the active runtime, retrieval, release, and ownership set
- documented the explicit application-service refactor roadmap, test strategy, and execution task ledger

## v1.2.0 - 2026-03-14

- clarified runtime and documentation boundaries around `context.db`, `memory_only` retrieval, and ownership routing
- added runtime baseline tooling for cold boot, warm boot, reindex, search, and queue replay measurement
- improved SQLite hot paths with busy timeout, ordered restore index, and outbox due-time indexing
- added SQLite FTS5 prototype over `search_docs` with trigger sync and lexical comparison coverage
- made FTS bootstrap rebuild crash-safe with `system_kv` schema marker retry
- removed duplicate benchmark retrieval work so trace latency reporting reuses one retrieval measurement
