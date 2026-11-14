---
component: init
---

## MODIFIED Requirements

### Requirement: init-names-missing-main <!-- base: a3a17a23f473285a -->
When the repository's current branch isn't `main` and neither a local `main` nor the remote's `main` exists, `clm init` SHALL still write its files and SHALL print, as the first line under `Next steps (yours):`, that CLM lands only on a branch named `main` (`trunk-is-main`) and the command that renames the current branch to it, `git branch -m <current> main`, with, when the repository has a remote, that the remote's default branch, CI triggers and branch protection keyed to the old name move with it. It SHALL print no such line when `main` exists locally or on the remote, and SHALL rename nothing itself.

#### Scenario: a repository on master
- GIVEN a git repository with one commit on `master`, no `main` and no remote
- WHEN `clm init` runs
- THEN it exits 0, writes its files, the first next step names `main` and `git branch -m master main`, and `master` is still the current branch

#### Scenario: an empty repository whose unborn branch is master
- GIVEN an empty git repository whose `HEAD` names `refs/heads/master`
- WHEN `clm init` runs
- THEN the first next step names `git branch -m master main`

#### Scenario: a remote on master
- GIVEN a git repository on `master` whose `origin` has `master` and no `main`
- WHEN `clm init --host git` runs
- THEN the first next step names `git branch -m master main` and says the remote's default branch, CI triggers and branch protection move with it

#### Scenario: main already there
- GIVEN a git repository on a feature branch whose local `main` exists
- WHEN `clm init` runs
- THEN no next step names `git branch -m`

## REMOVED Requirements

### Requirement: claude-md-points-at-guide <!-- base: e9cdc867e5675500 -->
### Requirement: claude-md-names-the-skills <!-- base: f687fbf909bda787 -->
