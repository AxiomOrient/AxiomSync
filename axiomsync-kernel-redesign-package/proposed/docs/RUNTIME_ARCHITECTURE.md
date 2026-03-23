# Runtime Architecture

## One sentence

AxiomSync stores immutable ingress, projects canonical evidence-bearing records, derives reusable knowledge, and serves replayable query interfaces.

## Layers

### 1. ingress ledger
- append-only
- dedupe-aware
- stores normalized envelopes and raw payload

### 2. canonical projection
- sessions
- actors
- entries
- artifacts
- anchors
- links

### 3. derived knowledge
- episodes
- claims
- procedures

### 4. derived retrieval
- FTS
- embeddings
- ranking caches

## Boundary

Outside AxiomSync:
- AxiomRelay
- browser extension
- clipboard/manual fallback
- browser-use repair
- axiomRams runtime execution state

Inside AxiomSync:
- knowledge state
- replay / rebuild
- query

## Rebuild rule

If projection, knowledge, and index disagree:
- ingress ledger wins first
- canonical projection wins second
- derived knowledge wins third
- indexes are disposable
