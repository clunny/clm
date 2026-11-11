---
component: orchestrator.run
---

## ADDED Requirements

### Requirement: run-fetch-retries-race
Every fetch of `origin` that `clm run <component>` and `clm run --finish <component>` make SHALL go through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments they fetch with today (`fetch --prune --quiet origin`), so a build's launch or finish whose fetch lost a race with another build of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, the command SHALL exit 2 as a failed fetch makes it exit today, with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`. No file under `crates/clm-run/src` SHALL run `git fetch` of `origin` other than through `fetch_retrying`; the fetch of the pinned superpowers copy, which names no remote of the clone, stays as it is.

#### Scenario: a run whose fetch loses a ref race
- GIVEN a ready, unclaimed component `b` and a free place, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm run b` runs
- THEN it exits 0, prints its launcher, `origin` holds `impl/b`, and the place `b` exists

#### Scenario: a run whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm run b` runs
- THEN it exits 2 with a message containing `cannot lock ref`, `origin` holds no `impl/b`, and no place `b` exists

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently
