---
name: release-branch-operations
description: Workflow skill that composes atomic git/release skills for main sync, release tag snapshots, and branch hygiene.
---

# Workflow / Release Branch Operations

## Purpose
Run release branch operations only through composable atomic skills.

## Expansion Order
1. `$git-preflight-clean-sync`
2. `$git-main-sync-ff`
3. Repeat `$git-release-tag-squash-commit` for each tag in order.
4. `$git-prune-branches-keep-main-release`

## Required Inputs
- `SOURCE_BRANCH` (string; required)
- `RELEASE_TAGS` (list; required)
- `REMOTE_NAME` (string; optional; default `origin`)

## Output
- Main sync result
- Release commit map (`tag -> commit`)
- Branch prune result
