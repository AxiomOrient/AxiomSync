# Release Runbook

출시 전에는 현재 표면이 실제로 빌드되고, 문서가 그 표면과 일치하는지만 확인한다.

## Preflight
- `README.md`와 `docs/`를 기준 문서로 사용한다.
- 삭제된 legacy command나 script를 릴리스 기준으로 다시 되살리지 않는다.
- root는 항상 새 임시 디렉터리로 검증한다.

## Required Gates
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- sink --help
cargo run -p axiomsync -- web --help
cargo run -p axiomsync -- mcp serve --help
```

## Runtime Smoke
```bash
tmp_root="$(mktemp -d)"
cargo run -p axiomsync -- --root "$tmp_root" init
cargo run -p axiomsync -- --root "$tmp_root" project doctor
cargo run -p axiomsync -- --root "$tmp_root" search "smoke"
```

## Release Decision
- commands above must succeed
- docs must describe only current commands and files
- no release guide may reference removed legacy commands or deleted helper scripts
