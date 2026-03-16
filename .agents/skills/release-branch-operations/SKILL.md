---
name: release-branch-operations
description: Workflow skill that composes atomic git/release skills for main sync, release tag snapshots, and branch hygiene.
---

# Workflow / Release Branch Operations

## Purpose
Run release branch operations only through composable atomic skills.

## Expansion Order
1. `$git-preflight-clean-sync`
2. `$git-configure-main-release-topology`
3. `$git-main-sync-ff`
4. Repeat `$git-release-tag-squash-commit` for each tag in order.
5. `$git-prune-branches-keep-main-release`

## Required Inputs
- `MAIN_BRANCH` (string; optional; default `main`)
- `RELEASE_BRANCH` (string; optional; default `release`)
- `SOURCE_BRANCH` (string; optional): setup may auto-detect when omitted
- `RELEASE_TAGS` (list; required)
- `REMOTE_NAME` (string; optional; default `origin`)
- `CACHE_PATHS` (list; optional): repository-specific cleanup paths for release snapshots

## Output
- Main sync result
- Release commit map (`tag -> commit`)
- Branch prune result
