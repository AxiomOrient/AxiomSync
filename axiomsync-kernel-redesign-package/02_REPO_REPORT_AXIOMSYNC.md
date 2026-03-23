# Repo Report — AxiomSync

## Verified public shape

### Public identity
AxiomSync presents itself as a **conversation-native local kernel** built around a single local SQLite `context.db`, with CLI, HTTP API, MCP, and a Rust web UI.

### Public surfaces currently bundled
The public README and command surface show that AxiomSync currently bundles:

- CLI
- HTTP API
- MCP server
- web UI
- connector ingestion
- connector sync
- connector repair
- connector watch
- connector serve

Supported connectors visible in public surfaces:
- ChatGPT Web
- Codex
- Claude Code
- Gemini CLI

## Public repo structure observed

Root:
- `Cargo.toml`
- `crates/`
- `docs/`
- `extensions/chatgpt/`
- scripts / configuration files

Visible crate directories:
- `crates/axiomsync`
- `crates/axiomsync-domain`
- `crates/axiomsync-kernel`
- `crates/axiomsync-mcp`
- `crates/axiomsync-store-sqlite`

But there is a contradiction:
- `docs/RUNTIME_ARCHITECTURE.md` describes a split workspace with these role crates
- root `Cargo.toml` publicly shows only `crates/axiomsync` as a workspace member
- `crates/README.md` publicly says there is only one Rust package

This is a material architecture inconsistency.

## Good parts

### 1. Kernel thinking is already present
The runtime and API docs consistently describe:
- parse
- normalize
- plan
- apply

That is the right kernel discipline.

### 2. SQLite local-first SSOT exists
The runtime architecture centers on a single local `context.db`.
That is the correct base for a local-first kernel.

### 3. Distinction between raw / canonical / derived is already present
The runtime architecture describes layers for:
- raw events
- canonical tables
- derived knowledge
- retrieval projection

That is the right direction.

### 4. ChatGPT selective extension direction is correct
The extension is public, selected-text only, explicit-send, with retry queue and provenance hints.
That is the right primary capture path for ChatGPT Web.

## Weak parts

### 1. AxiomSync still owns edge runtime concerns
The current command surface still includes:
- connector sync
- connector repair
- connector watch
- connector serve

These belong to a service edge daemon, not the kernel.

### 2. Extension-facing local server is still inside the kernel repo
The public app-shell README says the app-shell owns a browser-extension friendly ingest daemon.
That couples the kernel to one service runtime shape.

### 3. Docs and workspace layout contradict each other
A real generic kernel repo cannot leave this ambiguous.
The repo needs one clear truth:
- either one crate
- or a real workspace

### 4. Genericity is weaker than the name implies
Because connectors, watch loops, and service ingress remain inside AxiomSync,
the repo behaves more like an application monolith than a reusable kernel.

## Design conclusion

AxiomSync should keep:
- domain model
- kernel logic
- SQLite store
- query / MCP / thin HTTP surfaces

AxiomSync should eject:
- connector sync / watch / repair / serve
- browser extension runtime ownership
- service daemon responsibilities
- approval / spool / retry state
- product-facing operator UI


## Visible file / directory inventory used for this report

### Root-level visible directories / files
- `crates/`
- `docs/`
- `extensions/chatgpt/`
- `Cargo.toml`
- `README.md`

### Visible crate directories
- `crates/axiomsync`
- `crates/axiomsync-domain`
- `crates/axiomsync-kernel`
- `crates/axiomsync-mcp`
- `crates/axiomsync-store-sqlite`

### Visible app-shell files under `crates/axiomsync/src`
- `auth_store.rs`
- `command_line.rs`
- `connector_config.rs`
- `connectors.rs`
- `http_api.rs`
- `lib.rs`
- `llm.rs`
- `main.rs`
- `web_ui.rs`
- `config/`

### Visible kernel/domain/store files
- `crates/axiomsync-domain/src/domain.rs`
- `crates/axiomsync-domain/src/error.rs`
- `crates/axiomsync-domain/src/lib.rs`
- `crates/axiomsync-kernel/src/kernel.rs`
- `crates/axiomsync-kernel/src/logic.rs`
- `crates/axiomsync-kernel/src/ports.rs`
- `crates/axiomsync-kernel/src/lib.rs`
- `crates/axiomsync-mcp/src/lib.rs`
- `crates/axiomsync-mcp/src/mcp.rs`
- `crates/axiomsync-store-sqlite/src/context_db.rs`
- `crates/axiomsync-store-sqlite/src/schema.sql`
- `crates/axiomsync-store-sqlite/src/lib.rs`

### Visible extension asset
- `extensions/chatgpt/*`

This inventory is the visible implementation surface that informed the redesign.
