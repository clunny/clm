---
component: orchestrator
---

## ADDED Requirements

### Requirement: fetch-retries-race
Every fetch of `origin` that `clm-orchestrator` makes, in `clm next`, `clm next --wait`, `clm claim`, `clm claim backlog/<slug>`, `clm proposals`, `claim_local` and `taken_items`, SHALL go through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments it fetches with today (`fetch --prune --quiet origin`), so a fetch that lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, the command SHALL fail as a failed fetch fails it today, as a git failure (exit 2), with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`. No file under `crates/clm-orchestrator/src` SHALL run `git fetch` other than through `fetch_retrying`.

#### Scenario: a claim whose fetch loses a ref race
- GIVEN a ready, unclaimed component `b`, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm claim b` runs
- THEN it prints `impl/b`, exits 0, and `origin` holds `impl/b`

#### Scenario: a claim whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm claim b` runs
- THEN it exits 2 with a message containing `cannot lock ref`, and `origin` holds no `impl/b`

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently
