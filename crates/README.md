# Packages

이 저장소는 multi-crate Rust workspace다. 커널과 저장/질의/앱 shell 경계를 crate 수준에서 분리한다.

## Package Map
- [`axiomsync-domain`](./axiomsync-domain/Cargo.toml): canonical contracts, enums, deterministic helpers
- [`axiomsync-kernel`](./axiomsync-kernel/Cargo.toml): pure planning logic and application service
- [`axiomsync-store-sqlite`](./axiomsync-store-sqlite/Cargo.toml): SQLite repository and transaction apply adapter
- [`axiomsync-mcp`](./axiomsync-mcp/Cargo.toml): MCP adapter surface
- [`axiomsync`](./axiomsync/README.md): app shell, CLI, unified HTTP surface, web UI

## Out Of Repository
- web companion project
- mobile FFI companion project
- iOS and Android application shells

## Common Commands
```bash
cargo run -p axiomsync -- --help
cargo run -p axiomsync -- sink --help
bash scripts/quality_gates.sh
```

## Reader Path
- Start with [../README.md](../README.md)
- Runtime boundary: [`axiomsync`](./axiomsync/README.md)
- Runtime and CLI boundary: [`axiomsync`](./axiomsync/README.md)
- Contracts and architecture: [../docs/INDEX.md](../docs/INDEX.md)
