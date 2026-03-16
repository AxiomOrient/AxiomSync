# Release Operations Playbook (Skill-Composed)

## Atomic Skill Index
- `git-configure-main-release-topology`
  - file: `.agents/skills/git-configure-main-release-topology/`
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
    2. `git-configure-main-release-topology`
    3. `git-main-sync-ff`
    4. `git-release-tag-squash-commit` (repeat per tag)
    5. `git-prune-branches-keep-main-release`

## Runbook 0: Configure Main/Release Topology For An Existing Repo
Use this once when the repository does not already operate on `<MAIN_BRANCH>` and `<RELEASE_BRANCH>`.

1. Run `git-preflight-clean-sync`.
2. Run `git-configure-main-release-topology`.

Source selection order:
1. explicit `<SOURCE_BRANCH>`
2. remote default branch from `<REMOTE_NAME>/HEAD`
3. most-worked branch by reachable commit count

Reference commands:
```bash
git fetch <REMOTE_NAME> --prune
git symbolic-ref --short "refs/remotes/<REMOTE_NAME>/HEAD" 2>/dev/null | sed "s#^<REMOTE_NAME>/##"
for b in $(git for-each-ref --format='%(refname:short)' refs/heads refs/remotes | grep -Ev '^(<REMOTE_NAME>/HEAD|<REMOTE_NAME>/<MAIN_BRANCH>|<REMOTE_NAME>/<RELEASE_BRANCH>|<MAIN_BRANCH>|<RELEASE_BRANCH>)$' | sed "s#^<REMOTE_NAME>/##" | sort -u); do
  printf '%s\t%s\n' "$(git rev-list --count "$b")" "$b"
done | sort -nr | head -n1 | cut -f2
git switch -c <MAIN_BRANCH> <SELECTED_SOURCE_BRANCH>
git push -u <REMOTE_NAME> <MAIN_BRANCH>
git switch -c <RELEASE_BRANCH> <MAIN_BRANCH>
git push -u <REMOTE_NAME> <RELEASE_BRANCH>
```

## Runbook A: Integrate Source Into Main
1. Run `git-preflight-clean-sync`
- required: clean worktree
- command baseline:
```bash
git status --short --branch
git fetch <REMOTE_NAME> --prune
```
2. Run `git-main-sync-ff`
```bash
git checkout <MAIN_BRANCH>
git merge --ff-only <source-branch>
git push <REMOTE_NAME> <MAIN_BRANCH>
```

## Runbook B: Append One Release Commit From One Tag
1. Run `git-preflight-clean-sync` with required tag.
2. Run `git-release-tag-squash-commit` once.
```bash
git rev-parse <tag>
git checkout <RELEASE_BRANCH>
git rm -rf .
git clean -ffdx
git checkout <tag> -- .
rm -rf <CACHE_PATH_1> <CACHE_PATH_2>
git add -A
git commit -m "release(<tag>): squash snapshot from tag <tag>"
git push <REMOTE_NAME> <RELEASE_BRANCH>
```

## Runbook C: Rebuild Release Branch From Ordered Tags
Use this when release branch history must be regenerated as tag-only snapshots.

1. Preflight with `git-preflight-clean-sync`.
2. Create a temporary orphan branch.
3. For each tag in semver order, repeat `git-release-tag-squash-commit` logic on the temp branch.
4. Move `release` to rebuilt tip.
5. Force-push `<RELEASE_BRANCH>` only when explicitly approved.

Reference loop:
```bash
git checkout --orphan codex/release-rebuild
git rm -rf . || true
git clean -ffdx
for tag in $(git tag --list | awk '{orig=$0; norm=$0; sub(/^v/,"",norm); print norm" "orig}' | sort -V | awk '{print $2}'); do
  git rm -rf . || true
  git clean -ffdx
  git checkout "$tag" -- .
  rm -rf <CACHE_PATH_1> <CACHE_PATH_2>
  git add -A
  git diff --cached --quiet || git commit -m "release(${tag}): squash snapshot from tag ${tag}"
done
git branch -f <RELEASE_BRANCH> codex/release-rebuild
```

## Runbook D: Prune Non-Core Branches
Run `git-prune-branches-keep-main-release`.

```bash
for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do
  if [ "$b" != "<KEEP_BRANCH_1>" ] && [ "$b" != "<KEEP_BRANCH_2>" ]; then
    git branch -D "$b"
  fi
done
for rb in $(git for-each-ref --format='%(refname:short)' refs/remotes | grep '^<REMOTE_NAME>/' | sed "s#^<REMOTE_NAME>/##" | grep -Ev '^(HEAD|<KEEP_BRANCH_1>|<KEEP_BRANCH_2>)$'); do
  git push <REMOTE_NAME> --delete "$rb"
done
git fetch <REMOTE_NAME> --prune
```

## Acceptance Checks
- local branches: only `<MAIN_BRANCH>`, `<RELEASE_BRANCH>`
- remote branches: only `<REMOTE_NAME>/<MAIN_BRANCH>`, `<REMOTE_NAME>/<RELEASE_BRANCH>`
- release commits: only `release(<tag>): squash snapshot from tag <tag>` format
- latest release commit tag matches intended deployment tag
