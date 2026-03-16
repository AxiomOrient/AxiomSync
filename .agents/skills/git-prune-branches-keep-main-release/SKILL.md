---
name: git-prune-branches-keep-main-release
description: Delete every non-allowlisted branch locally and remotely; default allowlist is main and release.
---

# Git / Prune Branches Keep Main Release

## Purpose
Enforce two-branch topology by removing non-core branches.

## Default Program
```text
[stages: preflight>detect>act>verify>handoff | scope: refs(local,remote) | policy: allowlist-only,safety-first,deterministic-output | output: md(contract=v1)]
```

## Required Inputs
- `REMOTE_NAME` (string; optional; default `origin`)
- `KEEP_BRANCHES` (list; optional; default `main`, `release`)
- `DRY_RUN` (boolean; optional; default `false`)

## Procedure
1. Collect local and remote branch lists.
2. Build delete set = all branches not in allowlist.
3. Delete local branches from delete set.
4. Delete remote branches from delete set (excluding symbolic refs like `HEAD`).
5. Fetch prune and verify only allowlisted branches remain.

Remote delete set command:
```bash
git for-each-ref --format='%(refname:short)' refs/remotes \
  | grep '^origin/' \
  | sed 's#^origin/##' \
  | grep -Ev '^(HEAD|main|release)$'
```

## Safety Rules
- Never delete the currently checked-out branch.
- Never target symbolic remote refs.
- Stop and report if protected branch deletion is attempted.

## Output
- `DELETED_LOCAL`: list
- `DELETED_REMOTE`: list
- `REMAINING_LOCAL`: list
- `REMAINING_REMOTE`: list
