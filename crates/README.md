# Crates

Minimal crate index for the runtime repository.

## Entry Links
- [Root README](../README.md)
- [Docs Index](../docs/README.md)

## Modules

- [`axiomnexus-core`](./axiomnexus-core/README.md): domain/runtime engine, persistence, retrieval.
- [`axiomnexus-cli`](./axiomnexus-cli/README.md): operator/automation command surface.
- [`axiomnexus-mobile-ffi`](./axiomnexus-mobile-ffi/README.md): native mobile FFI boundary.

Out of scope in this repository:
- web viewer/server crate (moved to external project)
- iOS/Android app projects

## Run

```bash
cargo run -p axiomnexus-cli -- --help
```

Queue daemon (local operator workflow):

```bash
process-compose --log-file logs/process-compose.log -f process-compose.yaml up
```

Logs:
- `logs/process-compose.log`
- `logs/queue_daemon.log`

## Develop

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
