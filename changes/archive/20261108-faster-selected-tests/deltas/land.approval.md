---
component: land.approval
---

## ADDED Requirements
### Requirement: approve-retries-fetch
`clm approve` SHALL fetch `origin` through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`). When that returns `FetchError::Offline` it SHALL exit 1 saying it is offline, as it does today; when it returns `FetchError::Git` it SHALL exit 1 with git's message from the last fetch and no `offline`. Either way it SHALL append nothing.

#### Scenario: an approve whose fetch loses a ref race
- GIVEN `spec/x` adding `changes/x/change.md`, and a script named `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm approve spec/x` runs
- THEN it prints `approved <fingerprint> spec/x`, the writer's ledger holds the `approved` record, and its stderr doesn't contain `offline`

#### Scenario: an approve whose fetch keeps failing with origin reachable
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm approve spec/x` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, and the ledger has no new record

#### Needs
- vcs/fetch-retries

#### Serves
- capability/approve-intent
- capability/work-concurrently
