# Testing

현재 릴리스 라인에서 유지하는 검증 entrypoint만 적습니다.

## Required
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- connector --help
cargo run -p axiomsync -- mcp serve --help
```

## Regression Suites
- `crates/axiomsync/tests/renewal_kernel.rs`
- `crates/axiomsync/tests/http_and_mcp.rs`
- `crates/axiomsync/tests/domain_contracts.rs`
- `crates/axiomsync/tests/process_contract.rs`

## Opt-In
- `crates/axiomsync/tests/live_acceptance.rs`
  - ignored by default
  - enable only when live connector env vars are available

## Release Smoke
```bash
tmp_root="$(mktemp -d)"
cargo run -p axiomsync -- --root "$tmp_root" init
cargo run -p axiomsync -- --root "$tmp_root" project doctor
```

## Extension Smoke
```bash
cargo run -p axiomsync -- connector serve chatgpt --addr 127.0.0.1:4402
```

Then load `extensions/chatgpt` and verify `POST /` traffic reaches the local ingest server.
