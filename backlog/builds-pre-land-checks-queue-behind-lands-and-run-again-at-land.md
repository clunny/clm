---
title: "A build's pre-land check queues behind every other check on the clone, runs every test, and is then repeated by its land"
kind: bug
priority: P1
components: [land, ledger, orchestrator.run]
---

## Problem

Each build checks its code twice through one serialised queue:

1. In its worktree, `clm land --check-only` runs the whole `[land] check` with no `CLM_SELECTION` (`crates/clm-land/src/check_only.rs` passes `None`), under the clone's check lock `.clm/check.lock` (`land/check-lock`, `check-runs-one-at-a-time`). The build's prompt has it run again after any later fix.
2. At land, `clm land` runs `[land] check` again on the merge with `main`, with the tests `clm affected` selects, under the same lock.

So the early check, meant to catch fmt, lint and test failures in the build's own worktree before the land, is the slowest check in the queue: full, serialised with every land and every other build's pre-land check, and often run twice. With `/clm:build` running several builds at once, lands wait for a queue of full checks (388 s each in this repository, measured 2026-11-07). The work it proves is then thrown away: the land runs its own check even when `main` hasn't moved and the merged tree is the tree the build checked.

`check-runs-one-at-a-time` rejected a way to turn the lock off "Not now (`yagni`): nothing has shown the serialised checks to be the bottleneck; add it when one does." This is that evidence. Since then `tests-run-concurrently` and `CLM_SLOT` (`build-slots`) make every CLM-built project's tests able to overlap on one host, which was the lock's reason.

## Direction (chosen by the human, 2026-11-07)

Both parts, together:

- **The pre-land check is not serialised.** `clm land --check-only` runs `[land] check` without taking `.clm/check.lock`, in projects whose tests keep `tests-run-concurrently` (whether that is unconditional, follows the carried constraint, or a setting is for the design). It already compiles into the build's own target, not `.clm/land`. Builds then catch their failures in parallel, in their own worktrees.
- **A passing pre-land check counts for the land.** When `--check-only` passes on a clean working tree at a committed `HEAD` that contains `origin/main`, it appends a `checked` record for `HEAD`'s code key with the same command. A land whose merged tree is that tree (the branch up to date, `main` not moved since) then finds its code key checked and runs only `clm check --no-detectors` and the readers of checked code, taking no lock. When `main` has moved, the land runs its usual selected check on the merged tree, which stays the safety net.

## To settle in the design

- That a `checked` record written by a build's worktree check is as trustworthy as one written by a land: the same `clm`, the same machine, the same ledger, an exact code-key match, and a tree that was clean and committed when the check began and ended. What `ledger`'s Contract says a `checked` record means, and `legacy-code-key`'s refusal to accept another tree's record, both bear on this.
- Whether `--check-only` should also take a selection (`clm affected --base origin/main`) when it can't record, or stay a full run.
- What the build's prompt (`orchestrator.run`) says about re-running the check, and about catching fmt and lint first.
- Amending `check-runs-one-at-a-time`, and its interaction with `land-target-lock`, which the land check keeps.

Related: `code-lands-wait-in-arbitrary-order`, `a-land-check-that-builds-nothing-still-waits-for-one-that-does`, `queued-lands-each-run-a-full-check`, which matter less once most checking moves into the builds' worktrees.
