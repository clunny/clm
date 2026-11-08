---
component: init
---

## ADDED Requirements

### Requirement: init-keeps-a-remote
When the `clm.toml` that `clm init` leaves sets `host = "none"`, it SHALL call `clm-host`'s `keep_remote` after writing its files, so the push of the result to `main` that its next steps name (`init-next-steps`) has an `origin`, and SHALL name the remote's path in its output. It SHALL push nothing itself.

#### Scenario: empty repository with no remote
- GIVEN an empty git repository with no remote
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "none"`, `git remote get-url origin` prints the path of `clm/remote.git` under the common git directory, which has no branch, and the output names that path

#### Scenario: a remote already there
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`, and in turn one with no remote run with `--host git`
- WHEN `clm init` runs
- THEN the remotes are unchanged and no `clm/remote.git` exists

#### Needs
- host/none-keeps-a-remote

### Requirement: init-names-missing-main
When the repository's current branch isn't `main` and neither a local `main` nor the remote's `main` exists, `clm init` SHALL still write its files and SHALL print, as the first line under `Next steps (yours):`, that CLM lands only on a branch named `main` (`trunk-named-main`) and the command that renames the current branch to it, `git branch -m <current> main`, with, when the repository has a remote, that the remote's default branch, CI triggers and branch protection keyed to the old name move with it. It SHALL print no such line when `main` exists locally or on the remote, and SHALL rename nothing itself.

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
