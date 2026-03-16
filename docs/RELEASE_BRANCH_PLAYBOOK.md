# Release Operations Playbook (Skill-Composed)

## Atomic Skill Index
- `git-preflight-clean-sync`
  - file: `.agents/skills/git-preflight-clean-sync/`
- `git-main-sync-ff`
  - file: `.agents/skills/git-main-sync-ff/`
- `git-release-tag-squash-commit`
  - file: `.agents/skills/git-release-tag-squash-commit/`
- `git-prune-branches-keep-main-release`
  - file: `.agents/skills/git-prune-branches-keep-main-release/`

## Workflow Skill
- `release-branch-operations`
  - file: `.agents/skills/release-branch-operations/`
  - expands to atomic skills in this order:
    1. `git-preflight-clean-sync`
    2. `git-main-sync-ff`
    3. `git-release-tag-squash-commit` (repeat per tag)
    4. `git-prune-branches-keep-main-release`

## Runbook A: Integrate Source Into Main
1. Run `git-preflight-clean-sync`
- required: clean worktree
- command baseline:
```bash
git status --short --branch
git fetch origin --prune
```
2. Run `git-main-sync-ff`
```bash
git checkout main
git merge --ff-only <source-branch>
git push origin main
```

## Runbook B: Append One Release Commit From One Tag
1. Run `git-preflight-clean-sync` with required tag.
2. Run `git-release-tag-squash-commit` once.
```bash
git rev-parse <tag>
git checkout release
git rm -rf .
git clean -ffdx
git checkout <tag> -- .
rm -rf .axiomsync .axiomme
git add -A
git commit -m "release(<tag>): squash snapshot from tag <tag>"
git push origin release
```

## Runbook C: Rebuild Release Branch From Ordered Tags
Use this when release branch history must be regenerated as tag-only snapshots.

1. Preflight with `git-preflight-clean-sync`.
2. Create a temporary orphan branch.
3. For each tag in semver order, repeat `git-release-tag-squash-commit` logic on the temp branch.
4. Move `release` to rebuilt tip.
5. Force-push `release` only when explicitly approved.

Reference loop:
```bash
git checkout --orphan codex/release-rebuild
git rm -rf . || true
git clean -ffdx
for tag in $(git tag --list | awk '{orig=$0; norm=$0; sub(/^v/,"",norm); print norm" "orig}' | sort -V | awk '{print $2}'); do
  git rm -rf . || true
  git clean -ffdx
  git checkout "$tag" -- .
  rm -rf .axiomsync .axiomme
  git add -A
  git diff --cached --quiet || git commit -m "release(${tag}): squash snapshot from tag ${tag}"
done
git branch -f release codex/release-rebuild
```

## Runbook D: Prune Non-Core Branches
Run `git-prune-branches-keep-main-release`.

```bash
for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do
  if [ "$b" != "main" ] && [ "$b" != "release" ]; then
    git branch -D "$b"
  fi
done
for rb in $(git for-each-ref --format='%(refname:short)' refs/remotes | grep '^origin/' | sed 's#^origin/##' | grep -Ev '^(HEAD|main|release)$'); do
  git push origin --delete "$rb"
done
git fetch origin --prune
```

## Acceptance Checks
- local branches: only `main`, `release`
- remote branches: only `origin/main`, `origin/release`
- release commits: only `release(<tag>): squash snapshot from tag <tag>` format
- latest release commit tag matches intended deployment tag
