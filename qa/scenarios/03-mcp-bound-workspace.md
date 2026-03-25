# Scenario 03: MCP Bound Workspace

## Goal
Validate MCP usage as a bound workspace consumer:
- list tools
- search inside the bound workspace
- list runs inside the bound workspace
- reject missing workspace selector
- reject access outside the bound workspace

## Environment
- stdio MCP server
- one bound workspace id
- pre-seeded data from the HTTP scenario

## Inputs
- no extra fixture beyond the seeded HTTP scenario

## Steps
1. Start `mcp serve --transport stdio --workspace-id <bound id>`.
2. Call `tools/list`.
3. Call `search_cases` with Team A selector.
4. Call `list_runs` with Team A selector.
5. Call `list_runs` without `workspace_root`.
6. Call `resources/read` for a Team A case.
7. Attempt Team B search from the Team A bound session.

## Expected
- canonical tools only are exposed
- Team A search succeeds
- Team A run listing succeeds
- missing `workspace_root` returns validation error
- Team B access returns permission error

## Automated Entry
- `qa/bin/run-real-user-qa.sh mcp`
