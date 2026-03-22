# Runtime Architecture

현재 구조는 split workspace + single local runtime 모델이다.

## Workspace Roles
- `axiomsync-domain`: pure types, validation, deterministic helpers
- `axiomsync-kernel`: port traits, pure planning logic, application service
- `axiomsync-store-sqlite`: SQLite repository + transaction/apply adapter
- `axiomsync-mcp`: MCP request/response adapter
- `axiomsync`: CLI, HTTP, web UI, connector and config adapters, composition root

## Data Flow
1. external input enters through CLI, HTTP, MCP, or connector adapter
2. kernel parses and normalizes input into deterministic rows
3. kernel produces plan objects
4. SQLite adapter applies plans inside transaction boundaries
5. query surfaces read derived state and evidence-backed projections

## Storage Model
- raw input ledger: `raw_event`
- canonical projection: `workspace`, `conv_session`, `conv_turn`, `conv_item`, `artifact`, `evidence_anchor`
- derived knowledge: `episode`, `episode_member`, `insight`, `insight_anchor`, `verification`
- retrieval projection: `search_doc_redacted`
- operational state: `source_cursor`, `import_journal`, schema meta rows

## Boundary Rules
- pure logic stays in `axiomsync-kernel::logic`
- filesystem, HTTP, browser, SQLite, and LLM calls stay behind adapter modules
- kernel depends on ports, not concrete app/store implementations
- dry-run never mutates store state

## Retrieval Model
- projection search uses `search_doc_redacted`
- ranking combines exact/FTS/evidence/verification signals
- evidence fallback uses quoted evidence and bounded raw context, not direct raw transcript indexing
