# Runtime Architecture

현재 구조는 local-first universal agent memory kernel이다.

## Workspace Roles
- `axiomsync-domain`: pure types, validation, deterministic helpers
- `axiomsync-kernel`: port traits, pure planning logic, application service
- `axiomsync-store-sqlite`: SQLite repository + transaction/apply adapter
- `axiomsync-mcp`: MCP request/response adapter
- `axiomsync`: CLI, HTTP, web UI, composition root

## Data Flow
1. external input enters through CLI, HTTP, or MCP
2. kernel parses and normalizes input into deterministic rows
3. kernel produces plan objects
4. SQLite adapter applies plans inside transaction boundaries
5. query surfaces read view state and evidence-backed knowledge

## Storage Model
- raw record ledger: `raw_event`
- view projection:
  - thread view: `workspace`, `conv_session`, `conv_turn`, `conv_item`, `artifact`, `evidence_anchor`
  - execution view: `execution_run`, `execution_task`, `execution_check`, `execution_approval`, `execution_event`
  - document view: `document_record`
- knowledge layer: `episode`, `episode_member`, `insight`, `insight_anchor`, `verification`
- retrieval projection: `search_doc_redacted`
- operational state: `source_cursor`, `import_journal`, schema meta rows

public canonical noun은 `case`이고, 기존 `episode`/`runbook`는 compatibility alias로만 남긴다.

## Boundary Rules
- pure logic stays in `axiomsync-kernel::logic`
- filesystem, HTTP, browser, SQLite, and LLM calls stay behind adapter modules
- kernel depends on ports, not concrete app/store implementations
- dry-run never mutates store state
- external daemons write through `/sink/*` on the main web router
- capture/spool/retry/file watching은 kernel 밖 책임이다
- `program`/`state` 파일은 직접 정본으로 읽지 않고 external importer가 `document_snapshot` record로 전달한다

## Retrieval Model
- projection search uses `search_doc_redacted`
- ranking combines exact/FTS/evidence/verification signals
- evidence fallback uses quoted evidence and bounded raw context, not direct raw transcript indexing
- execution/document records는 raw ledger에 남지만 case derivation을 직접 오염시키지 않는다
