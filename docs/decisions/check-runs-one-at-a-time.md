---
title: "A clone runs one `[land] check` at a time, apart from a build's own check in its place"
date: 2026-11-05
status: accepted
---

A run of `[land] check` on a clone holds an exclusive advisory lock on `.clm/check.lock` in the main worktree's CLM root while the command runs (`land/check-lock`). A land's check, a `clm land --check-only` run outside a build place (slot 0) and `clm verify`'s local run take it, so no two of them run at the same time. A `--check-only` run in a build place, whose `CLM_SLOT` isn't 0, takes no lock: its slot keeps its tests apart (`build-slots`).

**Why.** The land lock (`land-target-lock`) guards the shared cargo target, so it covers only lands that compile into `.clm/land`. But a project's tests share more than a target: a fixed local port, a database, a browser profile. notespeed's browser tests bind a fixed port, so whichever of two overlapping checks started second failed for a reason unrelated to its branch, and a build spent its stop budget on retries or left its work unlanded. CLM can't know which host resources a project's tests take, so the runs that share slot 0 never overlap, and a failure caused by another such run's check can't happen.

**Why the lock is not enough, and what covers the rest.** Builds run the project's tests outside `[land] check` all the time, on every edit and in spot-checks, and one project's fixed-port tests still failed beside another build with the lock in place (GitHub issue #266). Serialising those runs would undo parallel builds. So every CLM-built project's tests must be able to run concurrently on one host, keying what they must know in advance on `CLM_SLOT` (`tests-run-concurrently`, `build-slots`). The rule binds every project as a carried constraint rather than relying on each to adopt it, and covers paths and servers as well as ports. Its costs are accepted: a project that breaks the rule fails intermittently, as it would with no lock.

**Why builds' checks take no lock.** With slots in place, the lock was a bottleneck: every build's pre-land `--check-only` queued behind every land's check and every other build's (`worktree-check-counts`). A build's slot already keeps its tests apart from every other run's, so its check runs beside the others.

**Why a lock of its own.** `land.lock` is held from the checkout to the push and only for lands on the shared target; widening it to every check would make a check-only run wait on gates and pushes and change which lands use the land tree. The check lock covers the check command alone. A land takes it after `land.lock` and `--check-only` takes only it, so the two can't deadlock.

**Cost.** Lands' checks on a clone run one after another, as do check-only runs outside a place. Building and testing the code, and each build's own pre-land check, run in parallel.

**Alternatives.**
- *Only warn in the method that fixed-port tests fail under parallel builds.* Rejected: it moves the defect to every adopter, and builds still waste their retries.
- *A setting to turn the lock off for projects whose tests share nothing.* Not now (`yagni`): the runs the lock still covers are lands' checks, which already queue behind each other on the land lock when they compile; add it when the serialised checks are shown to be a bottleneck.
- *Widening the lock to every test run a build makes.* Rejected: it serialises the builds themselves (`build-slots`).
