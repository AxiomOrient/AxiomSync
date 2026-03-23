# axiomsync

`axiomsync`는 split workspace 위의 app shell crate입니다. CLI, unified HTTP API, web UI, MCP binding을 조립하고 `axiomsync-kernel` service를 연다.

## Ownership
- app composition root
- CLI and HTTP entrypoints
- unified `web` server with `/sink/*`
- local auth adapter
- web UI
- `axiomsync-kernel` wiring

## Boundary
- `sink *`는 canonical kernel write surface다.
- projection/derive/query semantics는 계속 kernel이 소유한다.
- app shell은 request를 plan으로 만들거나 plan을 apply adapter에 넘기는 orchestration만 수행한다.
- capture/spool/retry/approval/browser integration은 이 crate 범위 밖이며 외부 edge repository가 소유한다.
- 이 crate는 external edge runtime을 직접 포함하지 않고 kernel-facing surface만 제공한다.

## Invariants
- 모든 의사결정 로직은 `Parse -> Normalize -> Plan -> Apply` 순서를 따른다.
- `plan-*`는 plan만 반환하고 `apply-*`만 mutation을 수행한다.
- ID와 해시는 canonical JSON 입력으로부터 결정론적으로 계산한다.
- 도메인 상태의 정본은 `context.db` 하나이며, auth는 별도 파일로 분리한다.
- Unix에서는 `auth.json`을 owner-only 권한으로 쓴다.
- `auth.json`은 workspace grant와 global admin token의 hash만 저장한다.
- HTTP admin/web route는 global admin token을 요구한다.
- 무인증 `sink` route는 loopback source address만 허용한다.

## Verification
```bash
cargo fmt --all --check
cargo clippy -p axiomsync --tests -- -D warnings
cargo test -p axiomsync --tests -- --nocapture
cargo run -p axiomsync -- sink --help
cargo run -p axiomsync -- web --help
```
