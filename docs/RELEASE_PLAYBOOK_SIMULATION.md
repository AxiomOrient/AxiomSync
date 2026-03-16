# Release Playbook Simulation Report

- execution date: 2026-03-16
- environment: local virtual git sandbox (`mktemp` + bare remote + working clone)
- objective: execute playbook runbooks with real git commands and verify final branch topology

## Scenario Executed
1. Create virtual repo + remote.
2. Create `main`, `dev` and push both.
3. Run Runbook A (`main` fast-forward from `dev`).
4. Create tags `v1.0.0`, `v1.1.0` on `main`.
5. Bootstrap `release` branch.
6. Run Runbook B twice (tag snapshot commits for `v1.0.0`, `v1.1.0`).
7. Create extra branch `feature/tmp`.
8. Run Runbook D (prune non-core branches).
9. Validate acceptance checks.

## Real Defects Found During Simulation
1. Remote prune loop attempted to delete `origin` pseudo-ref.
- failing command pattern: iterating `refs/remotes/origin` and deleting every non-allowlisted name.
- fix applied: filter only names starting with `origin/` before `sed`.

2. zsh glob expansion failure on `refs/remotes/origin/*`.
- failing pattern: unescaped wildcard caused shell-level expansion error.
- fix applied: switched to `refs/remotes` + `grep '^origin/'` pipeline, avoiding shell glob reliance.

## Final Pass Evidence
- `LOCAL_BRANCHES=main release`
- `REMOTE_BRANCHES=origin/main origin/release`
- `RELEASE_TOP=release(v1.1.0): squash snapshot from tag v1.1.0`
- `SIMULATION_RESULT=PASS`

## Topology Setup Simulation
1. Create a virtual repo whose working branch is `develop`, not `main`.
2. Add another active branch `feature/api`.
3. Point remote default branch (`origin/HEAD`) to `develop`.
4. Run the setup flow from the new `git-configure-main-release-topology` skill.
5. Verify `main` and `release` are created from `develop` without rewriting existing refs.

### Topology Setup Pass Evidence
- `DEFAULT_BRANCH=develop`
- `SELECTED_SOURCE_BRANCH=develop`
- `REMOTE_BRANCHES=origin/develop origin/feature/api origin/main origin/release`
- `SOURCE_TIP == MAIN_TIP == RELEASE_TIP`
- `TOPOLOGY_SIMULATION_RESULT=PASS`

## Conclusion
- updated playbook commands now execute successfully in virtual run.
- branch prune logic is shell-safe for zsh and excludes pseudo-refs correctly.
- topology setup flow can bootstrap `main` and `release` from a non-standard default branch.
