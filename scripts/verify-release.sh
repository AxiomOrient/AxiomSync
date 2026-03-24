#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

workspace_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ -z "$workspace_version" ]]; then
  echo "failed to read workspace version from Cargo.toml" >&2
  exit 1
fi

if ! grep -Fq "## v${workspace_version} -" CHANGELOG.md; then
  echo "CHANGELOG.md is missing release entry for v${workspace_version}" >&2
  exit 1
fi

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --nocapture
cargo test -p axiomsync-cli --test relay_interop relay_http_delivery_smoke_commits_only_after_both_apply_phases -- --nocapture
cargo run -p axiomsync-cli -- --help
cargo run -p axiomsync-cli -- sink --help
cargo run -p axiomsync-cli -- serve --help
cargo run -p axiomsync-cli -- mcp serve --help

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

cargo run -p axiomsync-cli -- --root "$tmp_root" init
cargo run -p axiomsync-cli -- --root "$tmp_root" project doctor
