---
status: applied
resolves: [code-lands-wait-in-arbitrary-order]
---

## Intent

Runs waiting for a clone's land lock or check lock take their turns in the order they began to wait. A land whose code is already checked, whose check runs only the tests that read the files it changed, goes ahead of them, so a spec or docs land no longer waits behind a full build and test run. A fixed cap of three priority turns in a row keeps those lands from starving the rest. A waiting run says how many go before it, and says it again as that count drops. Decision: `ordered-lock-queue`. Resolves `backlog/code-lands-wait-in-arbitrary-order`.

## Problem

`clm land` waits on `.clm/land.lock` when its check compiles into the shared target, and every `[land] check` run of a land, of a `--check-only` run outside a build place and of `clm verify` waits on `.clm/check.lock` (`land-target-lock`, `check-lock`). Both are plain advisory locks, which the operating system hands to waiters in no stated order. When `/clm:build` finishes several builds close together, a land that came first can be passed over, and a land that only runs a few reader tests waits behind one that builds and tests everything. The waiting line is printed once and says nothing of progress.

## Approaches

- **A ticket queue in front of each lock, with a lane for already-checked lands** (chosen). Turns go in ticket order. A land whose merged code key counts as checked and whose readers select a test takes a priority ticket. A ticket stays live while its owner holds a lock on it, so a killed run never holds a turn. The OS lock still provides exclusion. This lane's test is known before the lock and doesn't depend on the toolchain.
- **A lane based on measured check times.** Neutral across toolchains, but the ledger records no check durations yet. This is deferred to `slow-land-checks-go-unnoticed`, as the decision says.
- **A lane based on the size of the `clm affected` selection.** For unchecked code the selection is computed under the lock in the shared land tree, and a small selection in a compiled project still pays for the build.
- **Batching the queue into one check** (`queued-lands-each-run-a-full-check`). Larger, and it builds on an ordered queue, so it stays in the backlog.

## Outcome

`land` gains `lock-queue`. `check-lock` and `shared-check-runner` now point its waiting line and its waiting order at `lock-queue`. The Contract names the queues among what a land keeps in `.clm/`, and states the order. The Design adds **The queue** (tickets, turns, lanes, the cap and liveness) and updates the two waiting lines. `shared-check-runner` takes the lane as a parameter and prints its waiting line as `lock-queue` says. `verify` runs `[verify] check` through that runner, so it waits in the queue with an ordinary ticket. Its call site gains the `ordinary` lane argument, a forced fix the build of `shared-check-runner` makes and names in its pull request (`forced-fixes-cover-additions`), since no behaviour of `verify` changes.

## Constraints

- Nothing assumes cargo: both locks, the queue and the lane's test apply to whatever `[land] check` a project sets.
- Everything lives under the main worktree's `.clm/`, per clone (`land-places-ignored`).
- Exclusion stays the operating system's lock, so a run by an older `clm` on the same clone never overlaps another check. It only loses its place in the order.
- The waiting lines keep their prefixes (`waiting for another land's check on <path>`, `waiting for another check on <path>`), which tests and `verify`'s scenarios match.
