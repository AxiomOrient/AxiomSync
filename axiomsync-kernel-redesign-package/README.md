# AxiomSync Kernel Redesign Package

This package contains:

- access-scoped verification notes for the requested repos and commit heads
- repo-by-repo analysis reports
- cross-repo compatibility analysis
- a target-state redesign for AxiomSync as a generic knowledge kernel
- a storage and API spec
- a migration roadmap
- a patch-style package with proposed replacement files

## Important limitation

This is **not** a commit-perfect apply-ready patch against `AxiomSync@236e6b8`.

Why:
- `AxiomSync@236e6b8` could not be independently resolved with the available tools
- `axiomRelay@ed04595` could not be accessed
- `axiomRams@b32f34f` repo was accessible, but the exact commit hash could not be independently resolved

So this package is a **precise target-state patch package**:
- architecture is concrete
- schema is concrete
- sink contract is concrete
- file replacements are concrete
- but final application should be rebased onto a locally verified checkout
