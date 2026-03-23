# Migration and Patchset Plan

## Strategy

This is a boundary-correction migration, not an incremental polish pass.

The cleanest path is:

1. lock the kernel contract
2. split the workspace for real
3. remove edge runtime ownership from AxiomSync
4. replace the schema with a layered ledger/projection/knowledge/index model
5. add thin sink + query transports
6. rebase AxiomRelay and axiomRams integrations onto the new contract

## Recommended phases

### Phase 0 — commit-true local snapshot
Before applying anything:
- verify `AxiomSync@236e6b8` locally or use the current intended target snapshot
- verify the actual AxiomRelay and axiomRams heads locally

### Phase 1 — workspace truth
Make root `Cargo.toml` match the real workspace.

### Phase 2 — contract-first kernel
Add:
- `docs/API_CONTRACT.md`
- `schema/kernel_sink_contract.json`
- `crates/axiomsync-http`

### Phase 3 — schema replacement
Replace the SQLite schema with:
- ingress ledger
- canonical projection
- reusable knowledge
- derived indexes

### Phase 4 — command surface reduction
Remove from AxiomSync:
- connector sync
- connector watch
- connector repair
- connector serve

Keep only:
- local maintenance
- rebuild
- search
- MCP
- thin serve for sink/query if desired

### Phase 5 — AxiomRelay adaptation
AxiomRelay forwards only raw envelopes and reads query results.

### Phase 6 — axiomRams adaptation
axiomRams exports selected run evidence to the same sink.

## Risk notes

### Main technical risk
Old code paths may assume connector-specific logic inside the kernel repo.

### Main migration risk
Existing search and derivation code may rely on older projection tables.

### Main product risk
If AxiomSync keeps both old and new models too long, the kernel becomes harder to reason about.

## Recommendation
Prefer a decisive cutover with migration tooling over long dual-write ambiguity.
