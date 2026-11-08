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
