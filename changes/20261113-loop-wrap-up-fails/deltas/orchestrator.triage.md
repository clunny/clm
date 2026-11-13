---
component: orchestrator.triage
---

## ADDED Requirements

### Requirement: finish-delete-loses-race
When `origin` refuses a delete `clm draft --finish` makes of `spec/<change>` or `autosync/spec/<change>` (`draft-finish`) because the branch is no longer at the tip the fetch read, `clm draft --finish` SHALL read that branch on `origin` again and go on, as it does for a claim: a branch `origin` no longer has SHALL be skipped as one that doesn't exist, since another session or command deleted it after the fetch, and a branch whose tip moved SHALL be kept. It SHALL print no `removed` line for either, SHALL still do the rest of the finish, and the refusal SHALL NOT make it exit 2. When a delete fails and `origin` still has the branch at the tip the fetch read, the failure is git's and it SHALL exit 2.

#### Serves
- capability/work-concurrently

#### Scenario: autosync branch deleted since the fetch
- GIVEN the branches and worktree of "folded draft's branches deleted", and an executable `git` first on `PATH` that, before it runs the first `push` naming `origin`, deletes `autosync/spec/20261027-f` on `origin`, and runs the real `git` for every call
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, prints `removed origin/spec/20261027-f` and no `removed` line naming `autosync/spec/20261027-f`, the worktree and the local `spec/20261027-f` are gone, and `origin` has neither branch

#### Scenario: spec branch pushed since the fetch
- GIVEN the same, with a `git` that pushes a new commit to `spec/20261027-f` on `origin` instead
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 0, prints no `removed` line naming `origin/spec/20261027-f`, `origin/spec/20261027-f` is at the new commit, and `origin` has no `autosync/spec/20261027-f`

#### Scenario: a delete git fails
- GIVEN the same branches, with a `git` that fails every `push` naming `origin` with `fatal: unable to access` and exit 128, changing nothing on `origin`
- WHEN `clm draft --finish 20261027-f` runs
- THEN it exits 2
