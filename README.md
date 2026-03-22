# AxiomSync

Conversation-native local kernel for capturing agent sessions into a single SQLite `context.db`, projecting them into canonical threads and episodes, and serving them over CLI, HTTP, MCP, and a Rust-rendered web UI.

## Runtime Model
- Domain state: single SQLite store at `<root>/context.db`
- Auth grants: `<root>/auth.json`
- Connector config: `<root>/connectors.toml`
- Core pipeline: `Parse -> Normalize -> Plan -> Apply`
- Determinism: IDs and hashes are derived from canonicalized input JSON
- Public surfaces: CLI, HTTP API, MCP (`stdio` + HTTP), Maud web UI
- Connectors: ChatGPT Web, Codex, Claude Code, Gemini CLI

## Quick Start
```bash
cargo run -p axiomsync -- --help

cargo run -p axiomsync -- init
cargo run -p axiomsync -- connector ingest --connector codex --file /tmp/codex-events.json
cargo run -p axiomsync -- project rebuild
cargo run -p axiomsync -- derive
cargo run -p axiomsync -- search "timeout error"
cargo run -p axiomsync -- project auth-grant --workspace-root /repo/app --token secret-token
cargo run -p axiomsync -- web --addr 127.0.0.1:4400
```

## Connector Flow
- `connector ingest`: parse and normalize one JSON event or batch
- `connector sync codex`: fetch Codex app-server events into `raw_event`
- `connector watch gemini-cli --once`: import Gemini watch directory into `raw_event`
- `connector serve chatgpt|claude-code`: run a local ingest daemon for extension/hooks payloads
- `project rebuild`: regenerate `workspace`, `conv_session`, `conv_turn`, `conv_item`, `artifact`, `evidence_anchor`
- `derive`: segment episodes, run LLM extraction, synthesize verifications, rebuild `episode`/`insight`/`verification`
- `mcp serve`: expose `search_episodes`, `get_runbook`, `get_thread`, `get_evidence`, `search_commands`

## Release Docs
- Runtime/API: [`docs/API_CONTRACT.md`](./docs/API_CONTRACT.md)
- Architecture: [`docs/RUNTIME_ARCHITECTURE.md`](./docs/RUNTIME_ARCHITECTURE.md)
- Testing: [`docs/TESTING.md`](./docs/TESTING.md)
- Release checklist: [`docs/RELEASE_RUNBOOK.md`](./docs/RELEASE_RUNBOOK.md)

## Companion Asset
- ChatGPT capture extension: [`extensions/chatgpt`](./extensions/chatgpt)

## Verification
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- connector --help
cargo run -p axiomsync -- mcp serve --help
```

## ChatGPT Extension
Run the local ingest daemon before loading the browser extension:

```bash
cargo run -p axiomsync -- connector serve chatgpt --addr 127.0.0.1:4402
```

The extension posts selected ChatGPT message excerpts to `http://127.0.0.1:4402/` and retries failed deliveries from local extension storage.
