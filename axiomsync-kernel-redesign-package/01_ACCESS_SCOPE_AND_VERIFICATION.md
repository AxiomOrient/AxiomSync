# Access Scope and Commit Verification

## Requested targets

- `AxiomOrient/axiomRelay` commit `ed04595`
- `AxiomOrient/AxiomSync` commit `236e6b8`
- `AxiomOrient/axiomRams` commit `b32f34f`

## What was independently verified

### AxiomSync
Verified:
- public org listing includes `AxiomSync`
- repository page is accessible
- current public main README / docs / tree are accessible
- main commit history is accessible and shows `cf6df86` on the public history page

Not independently verified:
- exact commit `236e6b8`

### axiomRams
Verified:
- public repository page is accessible
- root tree, README, workspace `Cargo.toml`, and crate tree are accessible
- repository currently shows `1 Commit` on public main

Not independently verified:
- exact commit `b32f34f`

### axiomRelay
Not independently verified:
- repository page could not be fetched with the available tools
- repository name does not appear in the accessible public repo listing for the org
- exact commit `ed04595` could not be verified

## Consequence

This package distinguishes two layers:

1. **verified analysis**
   - AxiomSync public main
   - axiomRams public main
   - uploaded design / handoff documents

2. **non-verified targets**
   - exact `AxiomSync@236e6b8`
   - exact `axiomRams@b32f34f`
   - `axiomRelay@ed04595`

Any statement about those non-verified targets would be less exact than the evidence allows.

So the redesign and patch package are grounded in:
- verified public code that was accessible
- verified uploaded architecture documents
- explicit non-overreach where access was missing
