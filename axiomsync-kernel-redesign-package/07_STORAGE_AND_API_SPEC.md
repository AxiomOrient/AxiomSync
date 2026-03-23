# Storage and API Spec

## What AxiomSync should store

### 1. Immutable ingress ledger
The kernel must keep the exact evidence it received.

Store:
- source kind
- connector name
- external session / entry keys
- event kind
- observed and captured timestamps
- normalized envelope JSON
- raw payload JSON
- content hash
- dedupe key
- ingest status metadata

### 2. Canonical projection
The kernel should project raw ingress into the smallest reusable model.

#### `sessions`
A logical container.

Fields:
- `session_id`
- `session_kind`
- `external_session_key`
- `title`
- `workspace_root`
- `opened_at`
- `closed_at`
- `metadata_json`

#### `actors`
Fields:
- `actor_id`
- `actor_kind`
- `display_name`
- `stable_key`
- `metadata_json`

#### `entries`
An ordered evidence-bearing unit inside a session.

Fields:
- `entry_id`
- `session_id`
- `seq_no`
- `entry_kind`
- `actor_id`
- `parent_entry_id`
- `external_entry_key`
- `text_body`
- `started_at`
- `ended_at`
- `metadata_json`

#### `artifacts`
Fields:
- `artifact_id`
- `session_id`
- `entry_id`
- `artifact_kind`
- `uri`
- `mime_type`
- `sha256`
- `size_bytes`
- `metadata_json`

#### `anchors`
Evidence address inside an entry or artifact.

Fields:
- `anchor_id`
- `entry_id`
- `artifact_id`
- `anchor_kind`
- `locator_json`
- `preview_text`
- `fingerprint`

### 3. Derived reusable knowledge
This is the actual memory layer.

#### `episodes`
A coherent reusable unit such as:
- problem
- investigation
- fix
- decision
- workflow outcome

#### `claims`
A normalized reusable statement backed by evidence.

Examples:
- a bug root cause
- a design decision
- a configuration rule
- an implementation invariant

#### `procedures`
A reusable, evidence-backed how-to.
This replaces vague “runbook extraction”.

### 4. Rebuildable retrieval index
Derived only:
- FTS tables
- embeddings
- optional rerank caches

## What AxiomSync should not store as core truth

Do not make these kernel truth:
- spool state
- retry counters
- dead-letter queues
- approvals
- external auth refresh state
- operator task boards
- desktop app state
- web extension local queue state
- axiomRams canonical run state

## Kernel ingest contract

### `append_raw_events(batch)`
Input:
- list of raw envelopes

Behavior:
- validate
- dedupe
- append immutable receipts
- return accepted / rejected counts and ids

### `health()`
Reports:
- db availability
- migration version
- pending rebuild markers

### optional operator-only maintenance
- `rebuild_projection(scope)`
- `rebuild_derivations(scope)`
- `rebuild_index(scope)`

These are operator controls, not edge-capture coupling.

## Query contract

### Session / evidence retrieval
- `get_session(id)`
- `get_entry(id)`
- `get_artifact(id)`
- `get_anchor(id)`

### Search
- `search_entries(query, filters)`
- `search_episodes(query, filters)`
- `search_claims(query, filters)`
- `search_procedures(query, filters)`

### Reuse
- `find_fix(query, filters)`
- `find_decision(query, filters)`
- `find_procedure(query, filters)`

## Why this is better than `conv_*` everywhere

`conv_*` overfits AxiomRelay.

`run_*` overfits axiomRams.

`session_*` + `entry_*` preserves both.

That is the simplest generic kernel center that still matches the actual products.
