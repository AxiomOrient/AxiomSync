---
name: git-release-tag-squash-commit
description: Build one release branch commit from one tag snapshot and push it.
---

# Git / Release Tag Squash Commit

## Purpose
Append exactly one release commit derived from exactly one existing tag.

## Default Program
```text
[stages: preflight>act>verify>handoff | scope: branch(release) | policy: tag-source-only,deterministic-output | output: md(contract=v1)]
```

## Required Inputs
- `RELEASE_TAG` (string; required)
- `RELEASE_BRANCH` (string; optional; default `release`)
- `REMOTE_NAME` (string; optional; default `origin`)
- `CACHE_PATHS` (list; optional): cache/runtime paths to remove before commit when the repository needs that cleanup

## Procedure
1. Verify tag exists: `git rev-parse <RELEASE_TAG>`.
2. Verify `<RELEASE_BRANCH>` already exists. If not, run `git-configure-main-release-topology` first.
3. Checkout `<RELEASE_BRANCH>`.
4. Replace tree with tag snapshot:
- `git rm -rf .`
- `git clean -ffdx`
- `git checkout <RELEASE_TAG> -- .`
5. Remove any user-supplied `CACHE_PATHS` from worktree/index.
6. Commit:
- `git add -A`
- `git commit -m "release(<RELEASE_TAG>): squash snapshot from tag <RELEASE_TAG>"`
7. Push `<REMOTE_NAME> <RELEASE_BRANCH>`.

## Block Conditions
- Tag does not exist.
- Dirty worktree.
- Release branch does not exist.
- Nothing changed after snapshot apply.

## Output
- `RELEASE_COMMIT`: commit id
- `RELEASE_BRANCH_HEAD`: commit id
- `PUSH_RESULT`: success/failure with reason
