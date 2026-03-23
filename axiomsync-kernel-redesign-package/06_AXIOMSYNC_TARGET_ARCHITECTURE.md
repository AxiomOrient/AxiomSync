# AxiomSync Target Architecture

## Final design sentence

**AxiomSync is a local-first evidence-native knowledge kernel that stores immutable ingress, projects canonical sessions and entries, derives reusable episodes / claims / procedures, and serves replayable query surfaces.**

## Primary design choice

Keep conversation as first-class.

But do **not** make conversation the only container.

Use:

- `session`
- `entry`
- `artifact`
- `anchor`

as the canonical projection center.

Then specialize with:
- `session_kind = conversation`
- `session_kind = run`
- `session_kind = task`
- `session_kind = import`

This preserves conversation-native quality while remaining usable by axiomRams.

## Responsibility split

### AxiomSync owns
- immutable raw ingress ledger
- canonical projection
- derived reusable knowledge
- evidence anchoring
- replay / rebuild
- query surfaces
- MCP read tools/resources
- thin ingest sink

### AxiomSync does not own
- connector polling
- browser capture
- retry / dead-letter spool
- approval queue
- service UI / branding
- operator workflow state
- provider auth / refresh logic beyond local kernel access needs

## Internal crate layout

### `axiomsync-domain`
Pure types and invariants.

### `axiomsync-kernel`
Pure ingest / projection / derivation / query logic.

### `axiomsync-store-sqlite`
SQLite adapter and migration layer.

### `axiomsync-http`
Thin transport for sink + query.

### `axiomsync-mcp`
Thin MCP adapter for query tools/resources.

### `axiomsync-cli`
Operator CLI for local kernel maintenance.

## No product-edge crate in the kernel repo

Do not keep:
- connector watch
- connector serve
- connector sync
- connector repair
- extension runtime ownership

Those belong outside.

## Layer model

### Layer 1 — ingress ledger
Immutable raw envelopes and receipts.

### Layer 2 — canonical projection
Sessions, entries, artifacts, anchors, links.

### Layer 3 — derived knowledge
Episodes, claims, procedures.

### Layer 4 — derived retrieval
FTS / embeddings / ranking indexes.

Layer 4 is disposable and rebuildable.
