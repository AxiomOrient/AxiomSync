# QA Package

This folder is the real-user validation package for AxiomSync.

It groups together:
- scenario docs
- fixed input fixtures
- one automation entrypoint that executes the scenarios end-to-end
- a markdown report generated for each run

## Scope
- Validate the product as a user would operate it, not just as isolated unit or integration tests.
- Exercise clearly different environments:
  - local CLI-only usage
  - HTTP multi-workspace usage with scoped bearer auth
  - MCP usage bound to one workspace
  - relay-style loopback sink delivery with dedupe and cursor upsert

## Layout
- `qa/scenarios/`: manual QA guides and expected outcomes
- `qa/fixtures/`: stable request payloads used by the scenarios
- `qa/bin/run-real-user-qa.sh`: automation entrypoint

## Run
```bash
qa/bin/run-real-user-qa.sh
```

Run only selected scenarios:
```bash
qa/bin/run-real-user-qa.sh cli http
qa/bin/run-real-user-qa.sh mcp relay
```

Outputs are written under `target/qa/<timestamp>/`.

Each run produces:
- per-scenario working directories
- captured JSON and HTTP artifacts
- `report.md`

## Requirements
- `cargo`
- `jq`
- `curl`
- `sqlite3`
- Bash

## Scenarios
- [`01-cli-local-user.md`](./scenarios/01-cli-local-user.md)
- [`02-http-multi-workspace.md`](./scenarios/02-http-multi-workspace.md)
- [`03-mcp-bound-workspace.md`](./scenarios/03-mcp-bound-workspace.md)
- [`04-relay-loopback-sink.md`](./scenarios/04-relay-loopback-sink.md)

## Notes
- This package is intentionally separate from `scripts/verify-release.sh`.
- `scripts/verify-release.sh` stays a fast release gate.
- `qa/` is the deeper user-journey validation layer.
