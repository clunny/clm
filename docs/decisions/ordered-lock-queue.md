---
title: "Lands and checks take a clone's locks in arrival order, already-checked lands first"
date: 2026-11-08
status: accepted
---

`clm land` and `clm land --check-only` wait on two advisory locks, `.clm/land.lock` and `.clm/check.lock` (`land-target-lock`, `check-runs-one-at-a-time`). An operating system wakes a lock's waiters in no stated order, so a land that came first could be passed over, and a spec or docs land whose check runs a handful of reader tests waited behind a full build and test run. Each lock now has a queue in front of it (`land/lock-queue`): waiters take tickets, turns go in ticket order, and a land whose code key is already checked, whose check runs only the tests that read the files it changed (**Readers of checked code**), takes a priority ticket that goes first. After three priority turns in a row while another ticket waits, that ticket goes next, so a stream of spec lands can't starve a build's land.

**Why tickets locked by their owner.** A ticket is live while its run's process holds an advisory lock on it, which the system drops however the process dies. Nothing reads a process ID, so a reused ID can't keep a dead ticket alive, and the queue needs no cleanup beyond what the next reader does. The lock itself still provides exclusion: the queue only orders, so a run that skips it (an older `clm` on the same clone) loses its place in the order but never overlaps another check.

**Why "already checked" is the lane's test.** It is known before the lock and depends on no toolchain: the merged tree's code passed the project's check, and only the tests that read what the land changed run. It is the case the human asked for (2026-11-08). Other signals were weighed and left out:
- *Past check times from the ledger.* Neutral across toolchains, but `checked` records hold no duration today. Once `slow-land-checks-go-unnoticed` has the ledger record check times, a short measured check could join the lane.
- *Size of the `clm affected` selection.* For unchecked code it is computed in the shared land tree, under the lock, so it isn't known when the land queues. In a compiled project, a small selection still pays for the build, so it doesn't make a land cheap.
- *Small code lands, such as a build of one component.* Not asked for, and with no measured time they can't be told apart from slow ones.

**Why a fixed cap of three.** No project has asked to tune it (`yagni`). Three keeps a burst of spec lands moving while a waiting build lands within a few short checks.

**Alternatives.**
- *Tickets judged live by process ID.* Rejected: IDs are reused, and a check of another process's liveness differs by platform.
- *One queue for both locks.* Rejected: `--check-only` and `verify` take only the check lock, and a land takes the land lock first, so each lock needs its own order.
- *Batching the waiting lands into one check* (`queued-lands-each-run-a-full-check`). Left in the backlog: that work builds on this ordered queue.
