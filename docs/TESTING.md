# Testing

현재 릴리스 라인에서 유지하는 검증 entrypoint만 적습니다.

## Required
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture
cargo run -p axiomsync-cli -- --help
cargo run -p axiomsync-cli -- sink --help
cargo run -p axiomsync-cli -- serve --help
cargo run -p axiomsync-cli -- mcp serve --help
```

## Regression Suites
- `crates/axiomsync-cli/tests/replay_pipeline.rs`
- sink schema regression suite
- `crates/axiomsync-cli/tests/sink_contract.rs`
- `crates/axiomsync-cli/tests/http_and_mcp.rs`
- `crates/axiomsync-cli/tests/relay_interop.rs`
- `crates/axiomsync-cli/tests/public_surface_guard.rs`

## Sink Fixture Coverage
- `crates/axiomsync-cli/tests/fixtures/raw_event.chatgpt_selection.json`
- `crates/axiomsync-cli/tests/fixtures/raw_event.axiomrams_run_summary.json`
- `crates/axiomsync-cli/tests/fixtures/relay_packet_batch.json`
- `crates/axiomsync-cli/tests/fixtures/relay_expected_append_raw_events.json`
- `crates/axiomsync-cli/tests/fixtures/relay_expected_cursor_upserts.json`
- 검증 포인트:
  - `docs/contracts/kernel_sink_contract.json` schema validation
  - canonical append envelope ingest
  - nested payload projection (`selection.text`, `source_message.role`, `payload.artifacts`, `hints.entry_kind`)
  - evidence-first derivation
  - canonical case/thread/run/document/evidence retrieval
  - canonical HTTP/MCP helper surface only
  - relay fixture -> sink request translation parity
  - same-host loopback relay sink delivery sequence
  - sent commit allowed only after both sink apply phases
  - pending projection/derivation/index counts

## Release Smoke
```bash
tmp_root="$(mktemp -d)"
cargo run -p axiomsync-cli -- --root "$tmp_root" init
cargo run -p axiomsync-cli -- --root "$tmp_root" project doctor
```

## One-Shot Verification
```bash
./scripts/verify-release.sh
```

`verify-release.sh` 는 workspace regression 전체 외에 relay HTTP smoke를 한 번 더 명시 실행해 same-host interop gate를 고정한다.

focused relay smoke만 재실행하려면 아래 명령을 사용한다.

```bash
cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture
```
