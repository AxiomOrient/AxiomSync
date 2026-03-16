---
name: release-branch-operations
description: Maintain a two-branch workflow (main/release), update release with tag-based squash commits, and enforce branch hygiene.
---

# Release Branch Operations Skill

Use this skill when the user asks to:
- consolidate branches into `main`
- keep `main` and `release` as the only long-lived branches
- build or update `release` using tag-based squash commits only
- establish repeatable release branch governance

## Why
- Keeps integration and deployment concerns separate.
- Preserves release readability (one commit per release tag).
- Reduces branch sprawl and stale branch risk.

## Inputs required
- Target integration branch (usually `main`)
- Deployment branch name (`release`)
- Valid release tags (`vX.Y.Z` or project-specific semver tags)

## Procedure
1. Validate repository state
- `git status --short --branch`
- `git fetch origin --prune`
- Confirm tags with `git tag --list`

2. Update `main` to latest integrated state
- `git checkout main`
- Merge validated source branch into main (prefer `--ff-only` when possible)
- `git push origin main`

3. Build or update `release`
- `git checkout release` (or create if missing)
- For each release tag to publish:
  - `git rm -rf .`
  - `git clean -ffdx`
  - `git checkout <tag> -- .`
  - `rm -rf .axiomsync`
  - `git add -A`
  - `git commit -m "release(<tag>): squash snapshot from tag <tag>"`
- `git push origin release`

4. Enforce branch hygiene
- Delete local branches except `main`/`release`
- Delete remote branches except `origin/main`/`origin/release`

## Conditions and decisions
- If unrelated histories are detected:
  - Perform explicit merge strategy and record conflict policy.
- If working tree is dirty before branch surgery:
  - Stop and clean/stash/commit first.
- If tag is missing:
  - Do not create release commit; request tag creation first.
- If local runtime caches are staged (`.axiomsync`, `.axiomme`, etc.):
  - remove from index/worktree before commit.

## Output expectations
- `main` is latest and pushed.
- `release` has only tag-derived squash commits.
- only `main` and `release` remain as long-lived branches.
- operator report includes: what changed, which tags were used, and cleanup result.
