# Scenario 01: CLI Local User

## Goal
Validate the core local workflow with no server:
- initialize a root
- ingest raw events
- replay projection and derivation
- inspect health with `project doctor`
- query the rebuilt knowledge

## Environment
- single local root
- no auth grants
- no HTTP server
- no MCP server

## Fixtures
- [`../fixtures/cli-raw-events.json`](../fixtures/cli-raw-events.json)

## Steps
1. Run `init` on a fresh root.
2. Run `sink plan-append-raw-events` with the CLI fixture.
3. Run `sink apply-ingest-plan`.
4. Run `project plan-rebuild`.
5. Run `project apply-replay-plan`.
6. Run `project doctor`.
7. Run `query search-cases` with `workspace_root=/workspace/cli-demo`.

## Expected
- ingest plan contains 2 receipts
- replay creates 2 projected entries
- pending projection, derivation, and index counts are all 0
- search returns at least 1 case hit for `config drift`

## Automated Entry
- `qa/bin/run-real-user-qa.sh cli`
