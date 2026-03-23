# Repo Report — axiomRams

## Verified public shape

axiomRams is a **Rust + CLI + Tauri reference implementation** for a contract-first autonomous mission system package.

It publicly states that it:
- loads `program/` contracts
- creates, dry-runs, and resumes runs
- executes `plan -> do -> verify`
- halts on approvals
- persists canonical file state
- generates deterministic `run_id`
- exposes the same service through CLI and Tauri IPC

## Public workspace structure

Workspace members visible in `Cargo.toml`:
- `crates/rams-core`
- `crates/rams-cli`
- `src-tauri`

Repo tree also contains:
- `config/`
- `docs/`
- `fixtures/model`
- `frontend/`
- `reference/reference_autonomous_mission_system_v1`

## Architectural character

axiomRams is:
- file-first
- control-plane / runtime oriented
- approval-aware
- deterministic about run identity
- operator-console oriented
- intentionally minimal about transport

It explicitly keeps the contract surface but exports it through shared Rust services + CLI + Tauri instead of an HTTP server.

## Strengths

### 1. Clear source-of-truth discipline
The public README and bundled package docs center canonical state on files under a writable state root.

### 2. Correct responsibility boundary for a control-plane runtime
axiomRams owns:
- run creation / resume
- mission / flow execution
- deterministic verification
- approvals
- operator-facing CLI / desktop shell

That is a service/runtime responsibility, not a knowledge-kernel responsibility.

### 3. Low ceremony
It does not over-abstract transport. CLI + Tauri IPC are enough for this service.

## Implication for AxiomSync integration

AxiomSync should **not** absorb axiomRams canonical runtime state ownership.

axiomRams already has its own source of truth:
- `program/`
- `state/`
- `TASKS.json`
- `PROGRESS.md`
- `RESULT.json`
- `EVENTS.ndjson`

The right relationship is:

- axiomRams remains the source of truth for runtime execution state
- AxiomSync ingests selected raw or artifact evidence from axiomRams
- AxiomSync derives reusable memory / knowledge from that evidence

In other words:
- axiomRams = execution truth
- AxiomSync = reusable knowledge truth

That preserves single-writer discipline and avoids duplicated system-of-record semantics.


## Visible file / directory inventory used for this report

### Root-level visible directories / files
- `config/`
- `crates/`
- `docs/`
- `fixtures/model/`
- `frontend/`
- `reference/reference_autonomous_mission_system_v1/`
- `src-tauri/`
- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `package.json`
- `rust-toolchain.toml`

### Workspace members
- `crates/rams-core`
- `crates/rams-cli`
- `src-tauri`

### Visible crate tree
- `crates/rams-core/src`
- `crates/rams-core/Cargo.toml`
- `crates/rams-cli/src`
- `crates/rams-cli/Cargo.toml`

This inventory is enough to identify axiomRams as a control-plane/runtime project rather than a knowledge kernel.
