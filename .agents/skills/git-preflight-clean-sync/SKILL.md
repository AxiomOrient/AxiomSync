---
name: git-preflight-clean-sync
description: Validate clean worktree, sync refs, and verify required branches/tags before branch or release mutation.
---

# Git / Preflight Clean Sync

## Purpose
Run mandatory Git safety prechecks before merge, release, or branch-prune operations.

## Default Program
```text
[stages: preflight>detect>verify>handoff | scope: repo | policy: safety-first,deterministic-output | output: md(contract=v1)]
```

## Use When
- About to merge, reset, rebase, or rewrite branch topology.
- About to create release commits from tags.
- About to delete branches.

## Required Inputs
- `REMOTE_NAME` (string; optional): Remote name. Default `origin`.
- `REQUIRED_BRANCHES` (list; optional; shape: `{BRANCH}`): Branches that must exist.
- `REQUIRED_TAGS` (list; optional; shape: `{TAG}`): Tags that must exist.

## Procedure
1. Assert clean tree: `git status --short --branch` must have no staged/unstaged changes.
2. Sync refs: `git fetch <REMOTE_NAME> --prune`.
3. Capture branch map: local + remote branch list.
4. Verify required branches and tags.
5. Return BLOCKED if any required ref is missing.

## Output
- `PRECHECK_STATUS`: `ready|blocked`
- `MISSING_BRANCHES`: list
- `MISSING_TAGS`: list
- `REF_SNAPSHOT`: summary of local/remote branches and tags
