---
name: git-main-sync-ff
description: Fast-forward sync a target branch (default main) from a validated source branch and push it.
---

# Git / Main Sync FF

## Purpose
Update `main` to latest integrated line using fast-forward only.

## Default Program
```text
[stages: preflight>detect>act>verify>handoff | scope: repo | policy: ff-only,safety-first,deterministic-output | output: md(contract=v1)]
```

## Required Inputs
- `SOURCE_BRANCH` (string; required)
- `TARGET_BRANCH` (string; optional; default `main`)
- `REMOTE_NAME` (string; optional; default `origin`)

## Procedure
1. `git checkout <TARGET_BRANCH>`
2. `git merge --ff-only <SOURCE_BRANCH>`
3. `git push <REMOTE_NAME> <TARGET_BRANCH>`
4. Verify local and remote tip alignment.

## Block Conditions
- Non-fast-forward merge required.
- Dirty worktree.
- Source branch missing.

## Output
- `SYNC_STATUS`: `done|blocked`
- `TARGET_HEAD`: commit id
- `PUSH_RESULT`: success/failure with reason
