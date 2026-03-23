# Self Feedback

## Pass 1 — scope honesty
### Problem found
The request asked for three repos and exact commits.
Available access did not support exact verification of all three targets.

### Fix applied
Reduced claims strictly to:
- verified public AxiomSync surfaces
- verified public axiomRams surfaces
- verified uploaded design documents
- explicit non-verification for axiomRelay and exact commit hashes

### Why this is better
It keeps the report usable without faking certainty.

## Pass 2 — overfitted conversation schema
### Problem found
Earlier conversation-fit thinking leaned toward `conv_*` tables.
That would fit AxiomRelay better than axiomRams.

### Fix applied
Changed the canonical center to:
- session
- entry
- artifact
- anchor
- episode
- claim
- procedure

### Why this is better
It keeps conversation strong without making every other product pretend to be a chat transcript.

## Pass 3 — kernel bloat
### Problem found
It is tempting to let AxiomSync own spool, approvals, or connector runtime because that can look “convenient”.

### Fix applied
Kept AxiomSync strictly to:
- knowledge state
- replay / rebuild
- query
- thin ingest

### Why this is better
It preserves genericity and prevents overlap with both AxiomRelay and axiomRams.

## Pass 4 — knowledge model size
### Problem found
A full entity / graph ontology in core would make the kernel too heavy.

### Fix applied
Did not make entities or a graph system mandatory in v1.
Core reusable knowledge is:
- episodes
- claims
- procedures

### Why this is better
It stays small, concrete, and evidence-first.

## Pass 5 — patch realism
### Problem found
A “direct apply” patch would imply commit-perfect local source alignment that was not available.

### Fix applied
Packaged:
- target-state replacement files
- patch-style diffs
- migration notes
- clear instructions to rebase on a locally verified snapshot

### Why this is better
It is implementable without pretending to be byte-for-byte exact.
