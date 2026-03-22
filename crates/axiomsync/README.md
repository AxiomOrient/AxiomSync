# axiomsync

`axiomsync`는 split workspace 위의 app shell crate입니다. CLI, HTTP API, web UI, connector adapter를 조립하고 `axiomsync-kernel` service를 연다.

## Ownership
- app composition root
- CLI and HTTP entrypoints
- local connector/config/auth adapters
- web UI and browser-extension friendly ingest daemon
- `axiomsync-kernel` wiring

## Invariants
- 모든 의사결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- `dry-run`은 plan만 반환하고 파일 시스템을 변경하지 않는다.
- ID와 해시는 canonical JSON 입력으로부터 결정론적으로 계산한다.
- 도메인 상태의 정본은 `context.db` 하나이며, secrets/config는 별도 파일로 분리한다.

## Verification
```bash
cargo fmt --all -- --check
cargo clippy -p axiomsync --tests -- -D warnings
cargo test -p axiomsync --tests -- --nocapture
```
