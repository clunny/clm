---
component: orchestrator.triage
---

## ADDED Requirements

### Requirement: draft-fetch-retries-race
Every fetch of `origin` that `clm draft`, `clm draft --finish` and `clm draft --fold` make SHALL go through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments they fetch with today (`fetch --prune --quiet origin`), so a draft's start, finish or fold whose fetch lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, the command SHALL exit 2 as a failed fetch makes it exit today, with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`. `clm intake` keeps fetching through `clm-git`'s `fetch`, which tries again itself (`vcs/fetch-tries-again`), and `clm triage` through `orchestrator`'s `taken_items`. No file under `crates/clm-triage/src` SHALL run `git fetch` other than through `fetch_retrying` or `fetch`.

#### Scenario: a draft whose fetch loses a ref race
- GIVEN a `bug` item `s` on `origin/main`'s backlog, unclaimed, a free place, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm draft s` runs
- THEN it exits 0, prints its launcher, and `origin` holds `propose/s`

#### Scenario: a draft whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm draft s` runs
- THEN it exits 2 with a message containing `cannot lock ref`, `origin` holds no `propose/s`, and the clone holds no place

#### Needs
- vcs/fetch-retries
- vcs/fetch-tries-again

#### Serves
- capability/work-concurrently
