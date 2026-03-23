# Comparative Analysis — AxiomSync, AxiomRelay, axiomRams

## Functional center of each system

### AxiomSync
Should be:
- local-first
- evidence-native
- replayable
- queryable
- reusable across products

### AxiomRelay
Should be:
- capture / spool / retry / approval / forward
- connector and browser edge system
- primary source of conversation capture events

### axiomRams
Is:
- contract-first runtime / control plane
- file-first execution system
- operator-facing run orchestration system

## Why current AxiomSync shape is awkward

Current public AxiomSync already includes many edge/runtime concerns:
- connector operational commands
- local ingest server
- web UI
- extension-facing composition

That makes it overlap with AxiomRelay.

At the same time, axiomRams already has a strong runtime identity and file-state source of truth.
So if AxiomSync also tries to become a runtime service, it will overlap with axiomRams too.

That is the wrong center.

## Better compatibility model

### AxiomRelay -> AxiomSync
AxiomRelay sends:
- raw conversation events
- raw selections
- raw tool/result envelopes
- raw source cursors if needed by contract

AxiomSync stores:
- immutable ingress
- canonical session / entry / artifact / anchor projection
- derived episodes / claims / procedures

### axiomRams -> AxiomSync
axiomRams sends:
- run-event evidence
- result artifacts
- important operator decisions
- mission / flow outputs that merit reuse

AxiomSync stores:
- the same canonical evidence model
- derived reusable memory from runs

## The key compatibility insight

If AxiomSync uses chat-only tables like `conv_*`,
axiomRams integration becomes unnatural.

If AxiomSync uses control-plane-run tables as SSOT,
it duplicates axiomRams.

So the best kernel center is:

- **session**
- **entry**
- **artifact**
- **anchor**
- **episode**
- **claim**
- **procedure**

That lets conversations and runs coexist without either becoming the false universal model.

## Recommended connection model

### Ingest
- local: CLI or Unix socket first
- same-machine service: HTTP
- edge writes only raw packets

### Query
- MCP first for agents
- HTTP / CLI for operators and tests

### Single-writer rule
- AxiomRelay owns its spool state
- axiomRams owns its run state
- AxiomSync owns its knowledge state

No duplication of canonical write ownership across repos.
