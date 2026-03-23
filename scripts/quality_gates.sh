#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "[quality] formatting"
cargo fmt --all -- --check

echo "[quality] clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "[quality] workspace tests"
cargo test --workspace --quiet

echo "[quality] all gates passed"
