---
status: applied
from: backlog/clm-run-and-other-fetches-exit-on-a-ref-lock-race-with-a-build
resolves:
  - clm-run-and-other-fetches-exit-on-a-ref-lock-race-with-a-build
---

## Intent

Let every fetch of `origin` that CLM runs survive a lost ref race with another build of the same clone, as `clm land`, `clm reconnect` and `clm approve` already do. `clm run`, `clm run --finish`, `clm wrap-up`, `clm next`, `clm claim`, `clm proposals`, `clm draft`, `clm draft --finish` and `clm draft --fold` fetch through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`), and `clm-git`'s plain `fetch(root)`, which `clm rebase`, `clm verify`, the ledger's compaction and `clm intake` use, tries again the same way.

## Problem

During `/clm:build` with several builds running, `clm run <component>` exited 2 with `git fetch --prune --quiet origin: error: cannot lock ref 'refs/remotes/origin/impl/<other>': is at <sha> but expected <sha>`: a build in another worktree of the same clone moved that remote-tracking ref during the fetch, and an immediate retry succeeded. `fetch_retrying` was added for exactly this race, but only `land` and `land.approval` require it. `clm-run`, `clm-checkpoint`, `clm-orchestrator` and `clm-triage` each run their own plain `git fetch --prune --quiet origin`, and `clm_git::fetch` runs one plain `git fetch origin`, so any of them fails on a race that parallel builds make routine. No requirement states that these commands survive the race, so the fix needs written rules (`code-fix-needs-a-written-rule`).

## Approaches

1. **Each component's fetches go through `fetch_retrying`, and `clm_git::fetch` retries itself.** Chosen. It reuses the one retry loop `vcs` already offers (`reuse-first`), as `land/fetch-retry-in-layer` and `land.approval/approve-retries-fetch` did, keeps each command's fetch arguments and exit codes, and covers the plain-`fetch` callers (`clm rebase`, `clm verify`, the ledger's compaction, `clm intake`) with one `vcs` rule and no code change in their components.
2. **Only the four components the item names.** Rejected: `clm rebase`, `clm verify` and `clm intake` fetch `origin` in the same clone while builds run, and the item expects every fetch of `origin` to retry; one `vcs` rule closes them for the price of a few lines.
3. **One `vcs` rule with a scan failing any crate that runs `git fetch` outside `fetch_retrying`.** Rejected: the scan's test lives in `vcs` and would fail until four other components' code moved, which a build of `vcs` alone can't do (`node-reconciliation`).
4. **Make `fetch_retrying`'s `Offline` carry the last error so `fetch(root)` can return it.** Rejected: `land` and `land.approval` match on `FetchError`, so the change would be a contract change for nothing; `fetch(root)` returns the last run's error without the `ls-remote` probe instead.

## Outcome

- `vcs/fetch-tries-again` (added): `fetch(root)` runs `git fetch origin` up to three times with `fetch_retrying`'s waits, returning the last run's error, through `fetch_retrying`'s loop rather than a copy. `vcs`'s Contract names it.
- `orchestrator/fetch-retries-race`, `orchestrator.run/run-fetch-retries-race`, `orchestrator.checkpoint/wrap-up-fetch-retries-race`, `orchestrator.triage/draft-fetch-retries-race` (added): each component's `fetch --prune --quiet origin` goes through `fetch_retrying`, failing as a git failure with `FetchError`'s message when it gives up. Each component maps `FetchError` to its `Error::Git` with the error's display.
- `orchestrator/next-wait` (modified): its two scenarios with an unreachable `origin` say `origin can't be reached`, where they said git's reason; only the tests' citations and any assertion on git's text move.
- Code: `crates/clm-git/src/lib.rs` (`fetch` shares `fetch_retrying`'s loop), the private `fetch` helpers in `crates/clm-orchestrator/src/repo.rs`, `crates/clm-run/src/lib.rs`, `crates/clm-checkpoint/src/lib.rs`, and the three inline fetches in `crates/clm-triage/src/{draft,finish,fold}.rs`. Each test fakes `git` first on `PATH` as `crates/clm-git/tests/fetch_retries.rs` does.

## Constraints

- Every command keeps its fetch arguments and its exit code on a failed fetch; only the message of an offline failure becomes `origin can't be reached` where git's message stood, as `clm land` already says.
- An offline fetch now takes 3 seconds longer before failing (two retries after 1 and 2 seconds). `clm next --wait`'s retries while offline, the ledger's compaction (which ignores a failed fetch) and `clm rebase` pay that delay; none of them is waited on interactively at a rate where 3 seconds matters.

## Edge cases and workarounds

- `clm land`'s fetch of `main` after a refused push (`crates/clm-land/src/land.rs`) stays plain: any failure there already reports the refused push, which is the right report.
- `clm-host`'s fetch of a pull request's head (`refs/pull/<n>/head` into a local ref) updates no remote-tracking ref of `origin`, so it can't lose this race, and stays as it is.
- `clm run`'s fetch of the pinned superpowers copy names a URL, not `origin`, in a fresh repository of its own, and stays as it is.

## The human's answers

None bear on this item.

## Open questions

- **Time in land checks** (`land-checks-stay-fast`, raised by the architecture audit): each "keeps failing" scenario sleeps through the retries' 3 seconds, and each "loses a ref race" one through 1 second, about 16 seconds of sleeping across five crates' tests, and tests elsewhere that fetch an unreachable `origin` through `fetch` (such as `clm rebase`'s) grow 3 seconds each. Kept: `vcs/fetch-retries` and `land.approval/approve-retries-fetch` already pay the same waits, the crates' tests run in parallel, and the "keeps failing" scenarios are what prove each command's failure stays a git failure with the fetch's message. Moving them to `[verify]` would be the remedy if land checks slow noticeably.
