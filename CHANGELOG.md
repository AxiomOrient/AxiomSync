# Changelog

## Unreleased

- narrowed `client.rs` bootstrapping responsibilities and split resource ingest from filesystem convenience APIs
- reduced the documentation entrypoints to active runtime, release, and testing references
- aligned repository release metadata with the existing `v1.3.1` tag

## v1.3.1 - 2026-03-18

- release branch cut that included the pending workspace changes shipped as `v1.3.1`

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
