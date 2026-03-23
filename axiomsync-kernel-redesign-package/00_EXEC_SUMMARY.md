# Executive Summary

## Bottom line

The current public AxiomSync implementation is **valuable but over-scoped**.

It already has:
- a local SQLite-centered kernel story
- parse -> normalize -> plan -> apply discipline
- MCP / HTTP / CLI surfaces
- connector support for ChatGPT, Codex, Claude Code, Gemini CLI

But it also still owns too much:
- connector sync/watch/repair/serve
- extension-facing ingest daemon
- operational connector policy
- web UI composition concerns

That boundary makes AxiomSync less generic than it should be.

## Decision

AxiomSync should be rebuilt as a **generic local-first knowledge kernel** with these responsibilities only:

1. immutable raw ingress ledger
2. canonical projection into session / entry / artifact / evidence models
3. derived reusable knowledge:
   - episodes
   - claims
   - procedures
4. rebuildable retrieval indexes
5. narrow ingest contract
6. query surfaces through CLI / HTTP / MCP

Everything edge-specific stays outside:
- capture
- spool / retry / dead-letter
- approvals
- browser automation
- connector polling / watch / repair
- service branding and UX

## Why this matters

You want AxiomSync to serve both:
- **AxiomRelay** (conversation capture / forwarding)
- **axiomRams** (contract-first runtime / operator system)

That means AxiomSync cannot be ChatGPT-centric or service-centric.

It must become:
- **session-native**
- **evidence-native**
- **replayable**
- **connector-agnostic**
- **service-agnostic**

Conversation remains first-class, but not the only first-class shape.

## What changes most

The biggest design correction is this:

- previous conversation fit thinking leaned toward `conv_*` tables
- with AxiomRelay + axiomRams together, the better center is `session_*` + `entry_*`

This lets AxiomSync store:
- conversations
- selections
- tool results
- run events
- run artifacts
- decisions
- reusable procedures

without forcing non-chat data into fake chat-only tables.

## Package output

This package gives you:
- access-verified repo reports
- a target architecture
- a concrete schema
- narrow sink and query contracts
- a migration roadmap
- patch-style replacement files for AxiomSync
