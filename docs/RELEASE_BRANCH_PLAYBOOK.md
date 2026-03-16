# Branch and Release Operations Playbook

## 1. Why this model exists
- `main` is the integration truth: all validated work converges here.
- `release` is deployment truth: only versioned release snapshots live here.
- Keeping only two long-lived branches reduces drift, merge debt, and release mistakes.
- Release history stays readable because each deployable version is a single squash commit.

## 2. Operating contract (What)
- Allowed long-lived branches: `main`, `release` only.
- All feature or temporary branches are short-lived and must be deleted after merge.
- `release` must contain only commits shaped like:
  - `release(<tag>): squash snapshot from tag <tag>`
- Every release commit must map to a real git tag.

## 3. When to run which flow
- Daily integration:
  - Trigger: validated development changes are ready.
  - Action: integrate into `main`, push `main`.
- Release preparation:
  - Trigger: a production release candidate is approved on `main`.
  - Action: create release tag on `main` first.
- Release branch update:
  - Trigger: new release tag exists.
  - Action: add one squash snapshot commit for that tag on `release`.
- Branch hygiene:
  - Trigger: after integration/release update work.
  - Action: remove every branch except `main` and `release` (local + remote).

## 4. How to execute safely

### 4.1 Pre-flight checks
1. `git status --short --branch` must be clean.
2. Verify target tags exist: `git tag --list`.
3. Verify remote sync: `git fetch origin --prune`.

### 4.2 Make main the latest
1. Checkout main: `git checkout main`
2. Integrate latest validated line (example from dev): `git merge --ff-only dev`
3. Push: `git push origin main`

### 4.3 Create/advance release commit from a tag
1. Tag on main (example): `git tag -a v1.4.0 -m "release v1.4.0"`
2. Push tag: `git push origin v1.4.0`
3. Checkout release: `git checkout release`
4. Replace tree with tag snapshot:
   - `git rm -rf .`
   - `git clean -ffdx`
   - `git checkout v1.4.0 -- .`
   - `rm -rf .axiomsync`
5. Commit squash snapshot:
   - `git add -A`
   - `git commit -m "release(v1.4.0): squash snapshot from tag v1.4.0"`
6. Push release: `git push origin release`

### 4.4 Cleanup branches after work
- Local cleanup:
  - `for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do if [ "$b" != "main" ] && [ "$b" != "release" ]; then git branch -D "$b"; fi; done`
- Remote cleanup:
  - `for rb in $(git for-each-ref --format='%(refname:short)' refs/remotes/origin | sed 's#^origin/##' | grep -Ev '^(HEAD|main|release)$'); do git push origin --delete "$rb"; done`

## 5. Guardrails and failure conditions
- Never push `release` without a corresponding tag.
- Never keep temp branches after merge is done.
- If merge reports unrelated histories, stop and perform explicit merge plan with conflict strategy.
- If `.axiomsync/` or other local runtime cache appears in staged files, remove it before commit.
- If branch divergence is large, do not force-push blindly; validate commit lineage first.

## 6. Release day checklist
1. Main tests/gates pass.
2. Tag created on main and pushed.
3. Release squash commit created from that exact tag.
4. `origin/main` and `origin/release` both updated.
5. No extra local/remote branches except main/release.
6. Deployment automation points to `release` commit.
