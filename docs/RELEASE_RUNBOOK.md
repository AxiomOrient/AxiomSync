# Release Runbook

출시 전에는 현재 표면이 실제로 빌드되고, 문서가 그 표면과 일치하는지만 확인한다.

## Preflight
- `README.md`와 `docs/`를 기준 문서로 사용한다.
- 외부 edge repository의 capture/daemon 문서는 이 저장소 릴리스 계약으로 간주하지 않는다.
- 삭제된 legacy command나 script를 릴리스 기준으로 다시 되살리지 않는다.
- root는 항상 새 임시 디렉터리로 검증한다.

## Required Gates
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

## Runtime Smoke
```bash
tmp_root="$(mktemp -d)"
cargo run -p axiomsync-cli -- --root "$tmp_root" init
cargo run -p axiomsync-cli -- --root "$tmp_root" project doctor
```

## One-Shot Verification
```bash
./scripts/verify-release.sh
```

relay interop focused smoke:

```bash
cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture
```

## Release Decision
- commands above must succeed
- `./scripts/verify-release.sh` must pass from a clean checkout
- docs must describe only current commands and files
- docs must describe this repository's owned surface only
- relay interop docs and fixtures must match the same-host loopback sink contract
- `CHANGELOG.md` must contain the current workspace release entry
- no release guide may reference removed legacy commands or deleted helper scripts
