---
status: done
priority: P3
resolves:
  - web-site-suite-runs-when-web-site-is-unaffected
  - guard-timeout-tests-wait-out-real-budgets
  - clm-approve-calls-a-lost-ref-race-offline
---

## Intent

Three small cuts to what a land's check spends and how often it fails for nothing. `web/site`'s suite runs only when a land changes `web/site/` or runs every test, not on every `web` change. The guard's timeout tests prove the time budgets through the library with budgets of about a second, instead of waiting out the hook's real 20 and 30 seconds on every full run. And `clm approve`, like `clm land` and `clm reconnect`, tries a fetch that lost a ref race with another land again, and says it is offline only when `origin` can't be reached, through one retry in `clm-git` that all three share.

## Problem

- **The site's suite on every `web` change.** `ci/web-checks` runs `web/site/`'s suite whenever `web/`'s runs. `web` depends on `web.site`, not the reverse, so nothing else under `web/` changes what the site's tests check. On 2026-11-08 a one-test change to `clm-web-replica` spent 81 s of a 218 s check in `web/site`.
- **The guard's real budgets in tests.** Three tests in `crates/clm/tests/guard.rs` hang a fake `gh` or `git` and wait out `PUSH_BUDGET` (20 s) and `LOOKUP_BUDGET` (30 s): 30 of the 137 seconds of a full `cargo test` run measured on 2026-11-07. The scenarios run them through `clm hook guard`, the binary, so they can only be proven in real time.
- **`clm approve` offline on a ref race.** `clm approve` starts with `clm_git::fetch` and calls any failure being offline. A land in the same clone that moves a remote-tracking ref while the fetch runs makes git refuse it with `cannot lock ref`, though `origin` is reachable. `20261108-overlapping-lands-fails-under` fixed this for `clm land` and `clm reconnect` with a retry loop inside `clm-land` (`land/offline-land`), which `clm-approval` can't call, since `clm-land` depends on it.

## Approaches

**The site's suite.** (a) Run `web/site/` only when the selection names a file under it, or every test runs (chosen). (b) Give `web.site` its own selection through `check.affected` and drop the script's rule: the selection already reaches `web/site/` files, the script's rule is what adds it, so (a) is the whole fix. `capabilities/web/` keeps running whenever `web/`'s suite does: it drives the Worker as deployed, so a Worker change can break it (the human's choice on 2026-11-08).

**The guard's budgets.** Settled on 2026-11-07 while designing `20261107-faster-land-check`: the budgets become arguments of the library, `guard_within(event, budgets)`, with `guard` calling it with `Budgets::HOOK` (20 s, 30 s); the timeout scenarios call `guard_within` with about a second; a fast test checks the hook's budgets. Rejected: an environment variable or setting that shortens the budgets, since a safety hook's budget must not be shortenable in production. `crates/clm-land/tests/check_lock.rs` (6.4 s, scenarios with 2-second sleeps) is left as it is: its waits are the proof of the lock's ordering, and it saves little (the human's choice on 2026-11-08).

**The approve retry.** (a) Copy `clm-land`'s loop into `clm-approval`: two copies of one rule, against `reuse-first`. (b) Move the loop into `clm-approval`, which `clm-land` already depends on: approval is the wrong home for a git retry. (c) Put it in `clm-git` as `fetch_retrying(root, args)`, returning `Offline` or git's error, and have `clm land`, `clm reconnect` and `clm approve` use it (chosen): the git layer is where a fetch and its failure modes belong (`git-plumbing-layer`), both callers already depend on it, and the next caller gets the same rule.

## Outcome

- `ci/web-checks` and `ci/step-markers` reworded; the check script runs `web/site/` only for its own files or a full run.
- `plugin.hooks.guard/hook-budgets` added; `push-guard` and `release-guard` share the budgets it names, and their timeout scenarios call `guard_within` with a 1-second budget in `crates/clm-guard/tests/budgets.rs`.
- `vcs/fetch-retries` added; `land.approval/approve-retries-fetch` and `land/fetch-retry-in-layer` use it, each with `#### Needs vcs/fetch-retries`.

## Constraints

- No variable, setting or argument shortens the hook's budgets (`hook-budgets`).
- The retry waits stay 1 and 2 seconds, as `land/offline-land` states.
