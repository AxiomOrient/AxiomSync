# Scenario 02: HTTP Multi-Workspace

## Goal
Validate real HTTP usage with visibly different workspace scopes:
- seed two workspaces
- grant one workspace token per workspace
- grant one admin token
- prove allowed and denied flows over HTTP

## Environment
- one server root
- two workspace roots:
  - `/workspace/team-a`
  - `/workspace/team-b`
- one loopback HTTP server

## Fixtures
- [`../fixtures/http-workspace-a.json`](../fixtures/http-workspace-a.json)
- [`../fixtures/http-workspace-b.json`](../fixtures/http-workspace-b.json)

## Steps
1. Initialize a fresh root.
2. Ingest Team A fixture and Team B fixture.
3. Rebuild projection and derivation.
4. Grant workspace token for Team A.
5. Grant workspace token for Team B.
6. Grant admin token.
7. Start `serve`.
8. Execute these checks:
   - Team A token can search Team A
   - Team B token can search Team B
   - workspace token without `workspace_root` on `GET /api/runs` is rejected
   - Team A token cannot search Team B
   - admin token cannot use canonical workspace read routes

## Expected
- allowed same-workspace reads return `200`
- unscoped collection read returns `400`
- cross-workspace read returns `403`
- admin token on canonical workspace read returns `403`
- `auth.json` is owner-only on Unix

## Automated Entry
- `qa/bin/run-real-user-qa.sh http`
