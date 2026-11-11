---
component: orchestrator
---

## MODIFIED Requirements

### Requirement: next-wait <!-- base: 49b8dc881f075ebe -->
`clm next --wait` SHALL look for work as `clm next` does and, while a look would print nothing, wait `--interval <minutes>` (a positive decimal number, default 5) and look again, fetching `origin` at each look, until a look would print at least one component; then it SHALL print what that look prints, as `clm next` would, and exit 0. When the first look fails, it SHALL exit as `clm next` would, 1 when the work isn't possible and 2 when git fails, with its reason. When a later look fails, it SHALL print `clm next failed: <reason>` to stderr and wait for the next look, never exiting for it. An `--interval` that isn't a positive number SHALL exit 1 before the first look, saying so. `--interval` without `--wait` SHALL be refused as a usage error. `clm next` without `--wait` SHALL behave as before.

#### Scenario: work arrives
- GIVEN a clone where no component is ready, and `clm next --wait --interval 0.01` running
- WHEN a commit adding an uncited requirement to component `a` is pushed to `origin/main`
- THEN it prints `a` and exits 0

#### Scenario: work already there
- GIVEN a clone where component `a` is ready and unclaimed
- WHEN `clm next --wait` runs
- THEN it prints `a` at once and exits 0, as `clm next` would

#### Scenario: a place frees
- GIVEN ready component `a`, `max_builds` 1, one place that counts for a live claim `origin/impl/b`, and `clm next --wait --interval 0.01` running
- WHEN `impl/b` lands on `origin/main`
- THEN it prints `a` and exits 0

#### Scenario: a later look fails
- GIVEN `clm next --wait --interval 0.01` running after a first look that printed nothing, and `origin` then unreachable
- WHEN it looks again, and `origin` is reachable again with component `a` ready
- THEN it prints `clm next failed: ` with the fetch's reason, `origin can't be reached`, to stderr, keeps waiting, then prints `a` and exits 0

#### Scenario: first look fails
- GIVEN a clone whose `origin` can't be fetched
- WHEN `clm next --wait` runs
- THEN it exits 2 saying `origin can't be reached`, as `clm next` does

#### Scenario: bad interval
- GIVEN any clone
- WHEN `clm next --wait --interval 0` runs
- THEN it exits 1 saying the interval must be a positive number, without fetching

#### Serves
- capability/build-from-spec

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
