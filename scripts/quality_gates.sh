#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v cargo-audit >/dev/null 2>&1; then
  echo "cargo-audit is required (install: cargo install --locked cargo-audit)" >&2
  exit 1
fi

echo "[quality] dependency audit"
cargo audit --deny unsound --deny unmaintained --deny yanked

echo "[quality] formatting"
cargo fmt --all -- --check

echo "[quality] clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "[quality] workspace tests"
cargo test --workspace --quiet

echo "[quality] all gates passed"
