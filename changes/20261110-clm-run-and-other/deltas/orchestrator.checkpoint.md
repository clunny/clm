---
component: orchestrator.checkpoint
---

## ADDED Requirements

### Requirement: wrap-up-fetch-retries-race
`clm wrap-up` SHALL fetch `origin` through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments it fetches with today (`fetch --prune --quiet origin`), so a wrap-up whose fetch lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, it SHALL fail as a failed fetch makes it fail today, removing and deleting nothing, with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`.

#### Scenario: a wrap-up whose fetch loses a ref race
- GIVEN a clone with nothing to clean up, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm wrap-up` runs
- THEN it prints `wrap-up ok` and exits 0

#### Scenario: a wrap-up whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm wrap-up` runs
- THEN it fails with a message containing `cannot lock ref`, and doesn't print `wrap-up ok`

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently
