# Testing

현재 릴리스 라인에서 유지하는 검증 entrypoint만 적습니다.

## Required
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- sink --help
cargo run -p axiomsync -- serve --help
cargo run -p axiomsync -- mcp serve --help
```

## Regression Suites
- `crates/axiomsync/tests/kernel_redesign.rs`
- sink schema regression suite
- `crates/axiomsync/tests/http_and_mcp_v2.rs`

## Sink Fixture Coverage
- `crates/axiomsync/tests/fixtures/raw_event.chatgpt_selection.json`
- `crates/axiomsync/tests/fixtures/raw_event.axiomrams_run_summary.json`
- 검증 포인트:
  - `docs/contracts/kernel_sink_contract.json` schema validation
  - canonical append envelope ingest
  - nested payload projection (`selection.text`, `source_message.role`, `payload.artifacts`, `hints.entry_kind`)
  - evidence-first derivation
  - canonical case/thread/run/document/evidence retrieval
  - canonical HTTP/MCP helper surface only
  - pending projection/derivation/index counts

## Release Smoke
```bash
tmp_root="$(mktemp -d)"
cargo run -p axiomsync -- --root "$tmp_root" init
cargo run -p axiomsync -- --root "$tmp_root" project doctor
```
