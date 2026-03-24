# Repo Cleanup Guide

This guide defines a simple repository cleanup standard that stays useful beyond one project.

## Core Rule

Place files by ownership and lifecycle, not by convenience.

- If something belongs to one crate, keep it near that crate.
- If something is shipped code, keep it under `crates/`.
- If something is human-facing documentation, keep it under `docs/` or `README.md`.
- If something is repository tooling, keep it under `tools/`.
- Do not create new root folders unless they are clearly cross-repo in purpose.
- Keep human-facing guidance and agent-facing guidance separate.

## Human vs Agent

- Humans should need only `README.md` and `docs/`.
- Agent-only operating rules should live in `AGENTS.md` or repository skills.
- Do not create duplicate human docs just to help agents.

## Inputs, Tools, Outputs

Use three questions:

1. What is the input?
2. What tool transforms it?
3. What is the checked-in output?

Default placement:

- Project-local input: near the owning crate or package
- Repo-level tool: under `tools/`
- Checked-in generated output: near the code that compiles against it

## Local Mapping

- Input:
  - `crates/codexus-core/protocol-inputs/openai/.../protocol/common.rs`
  - `crates/codexus-core/protocol-inputs/openai/.../protocol/v2.rs`
- Generator:
  - `tools/xtask`
- Output:
  - `crates/codexus-core/src/protocol/generated/`

This means protocol input and output now share the same crate ownership boundary.

## Root Policy

The repository root should stay limited to:
- `crates/`
- `docs/`
- `tools/`
- `README.md`
- workspace manifests and ignore files
- agent control files such as `AGENTS.md`

Do not leave temporary spec folders, copied upstream trees, or generated outputs at the root.

## Cleanup Checklist

Before adding or moving files:

1. Ask which crate or boundary owns the file.
2. Keep inputs and outputs close to the owning crate when they are not truly workspace-global.
3. Keep generated outputs out of the repository root.
4. Keep human-facing explanations in `docs/` only.
5. Keep agent-only procedure in `AGENTS.md` or repository skills.
6. Remove Finder/editor junk and stale duplicate documents.

## Verification

After structure or generator changes, run:

```bash
cargo run -p xtask -- protocol-codegen
cargo fmt --all --check
cargo test --workspace
```
