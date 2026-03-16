---
name: git-configure-main-release-topology
description: Establish main/release branch topology for a repository that does not already use it.
---

# Git / Configure Main Release Topology

## Purpose
Bootstrap a repository into a two-branch operating model: one integration branch and one deployment branch.

## Default Program
```text
[stages: preflight>detect>act>verify>handoff | scope: repo | policy: idempotent-setup,safety-first,deterministic-output | output: md(contract=v1)]
```

## Required Inputs
- `REMOTE_NAME` (string; optional; default `origin`)
- `MAIN_BRANCH` (string; optional; default `main`)
- `RELEASE_BRANCH` (string; optional; default `release`)
- `SOURCE_BRANCH` (string; optional): explicit source branch for bootstrapping
- `SOURCE_DISCOVERY` (string; optional; default `explicit-then-remote-default-then-most-worked`)

## Procedure
1. Run preflight: clean worktree and `git fetch <REMOTE_NAME> --prune`.
2. If `<MAIN_BRANCH>` already exists locally or remotely, treat topology as partially configured and do not rewrite it.
3. If `<MAIN_BRANCH>` does not exist, choose source branch in this order:
- explicit `SOURCE_BRANCH`
- `<REMOTE_NAME>/HEAD` default branch
- the non-release branch with the highest reachable commit count
4. Create `<MAIN_BRANCH>` from the chosen source branch and push it.
5. If `<RELEASE_BRANCH>` does not exist, create it from `<MAIN_BRANCH>` and push it.
6. Verify local and remote refs now expose `<MAIN_BRANCH>` and `<RELEASE_BRANCH>`.
7. Report the chosen source branch and any manual follow-up required.

## Detection Commands
Remote default branch:
```bash
git symbolic-ref --short "refs/remotes/<REMOTE_NAME>/HEAD" 2>/dev/null | sed "s#^<REMOTE_NAME>/##"
```

Most-worked branch heuristic:
```bash
for b in $(git for-each-ref --format='%(refname:short)' refs/heads refs/remotes | grep -Ev '^(<REMOTE_NAME>/HEAD|<REMOTE_NAME>/<MAIN_BRANCH>|<REMOTE_NAME>/<RELEASE_BRANCH>|<MAIN_BRANCH>|<RELEASE_BRANCH>)$' | sed "s#^<REMOTE_NAME>/##" | sort -u); do
  printf '%s\t%s\n' "$(git rev-list --count "$b")" "$b"
done | sort -nr | head -n1 | cut -f2
```

## Safety Rules
- Never rewrite an existing `<MAIN_BRANCH>` in this setup skill.
- Never rewrite an existing `<RELEASE_BRANCH>` in this setup skill.
- If both target branches already exist but point to unrelated histories, stop and hand off.
- After creating `<MAIN_BRANCH>`, update the repository host's default branch manually if it still points elsewhere.

## Output
- `SETUP_STATUS`: `configured|already-configured|blocked`
- `SELECTED_SOURCE_BRANCH`: branch name or `none`
- `CREATED_BRANCHES`: list
- `FOLLOW_UPS`: list
