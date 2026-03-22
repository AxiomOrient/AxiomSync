# API Contract

이 문서는 현재 릴리스 라인이 실제로 보장하는 public surface만 적습니다.

## Repository Boundary
- 이 저장소는 AxiomSync kernel, CLI, HTTP API, MCP server, local web UI를 소유한다.
- 선택적 companion asset은 `extensions/chatgpt` 하나뿐이다.
- 과거 v3 resource/event/link runtime, migration, release-pack surface는 현재 릴리스 범위에 포함되지 않는다.

## Runtime Files
- canonical store: `<root>/context.db`
- auth grants: `<root>/auth.json`
- connector config: `<root>/connectors.toml`

## Core Contract
- 모든 결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- `dry-run`은 apply를 실행하지 않고 plan payload만 반환한다.
- stable id/hash/path는 canonicalized input에서 결정론적으로 계산한다.
- raw transcript 전체를 직접 FTS하지 않고 `search_doc_redacted`와 evidence fallback을 사용한다.

## CLI Surface
- `axiomsync init`
- `axiomsync connector ingest`
- `axiomsync connector sync`
- `axiomsync connector repair`
- `axiomsync connector watch`
- `axiomsync connector serve`
- `axiomsync project rebuild`
- `axiomsync project purge`
- `axiomsync project doctor`
- `axiomsync project auth-grant`
- `axiomsync derive`
- `axiomsync search`
- `axiomsync runbook`
- `axiomsync mcp serve`
- `axiomsync web`

## HTTP Surface
- `GET /health`
- `GET /`
- `GET /episodes/{id}`
- `GET /connectors`
- `POST /ingest/{connector}`
- `POST /project`
- `POST /derive`
- `GET /api/episodes`
- `GET /api/runbooks/{id}`
- `GET /api/threads/{id}`
- `GET /api/evidence/{id}`
- `POST /mcp`

## MCP Surface
- transports: `stdio`, HTTP
- methods:
  - `initialize`
  - `roots/list`
  - `resources/list`
  - `resources/read`
  - `tools/list`
  - `tools/call`

## Connector Contract
- supported connectors: `chatgpt`, `codex`, `claude_code`, `gemini_cli`
- ChatGPT browser capture uses the local companion extension and `connector serve chatgpt`
- Codex sync uses configured app-server JSON fetch
- Claude Code uses local hook/ingest payloads
- Gemini CLI uses watch-directory repair/ingest

## Auth And Scope
- main HTTP and web surface requires bearer auth for workspace-scoped reads and writes
- MCP HTTP binding enforces workspace scope
- dedicated connector ingest daemons are local-only helper endpoints and do not use bearer auth
