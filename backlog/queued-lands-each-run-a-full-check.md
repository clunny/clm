---
title: "Lands waiting in the queue each run a full check of their own, one after another"
kind: idea
components: [land, land.approval, ledger, gate]
---

When N lands queue for a clone's check, each runs the full check on its own merge with `main`, one after another, so the last waits N checks. Most of those checks pass. Merge queues (GitHub's merge queue, bors) instead check one tree holding `main` and all the queued branches, land them all when it passes, and split the batch in halves when it fails, finding the failing branch in about log N checks. That saves up to N-1 checks with no more disk or compute at once, unlike checking speculative trees in parallel, which needs N builds at the same time.

## Settled (2026-11-08, with the human)

- **Order.** Second of three: after `20261108-toolchain-neutral-land-builds` (a land re-checks only where its branch and what `main` gained overlap, `land/overlap-recheck`), and before finer test selection (`finer-test-selection`). The overlap rule removes the re-checks of independent changes; batching is for the lands whose changes do affect each other, which still run a check.
- **What a batch holds.** Only lands that would run a check: a land whose code key counts as checked, or whose overlap selects nothing, lands at once as it does now and never waits for a batch. A batch's selection is the union of its lands' own selections (their overlaps with `main`, or their own selection when they have no full check), plus whatever their changes select against each other.
- **CLM's part, all of it.** The queue, forming a batch, splitting it on failure, landing each branch with its own `landed` record, and keeping each branch's approval valid are CLM's, since a project's check script can't see other lands. The project's part is only what `land-checks-stay-fast` already asks: a check that runs on whatever tree it's given, and tests that are deterministic.
- **Flaky tests.** A flaky test makes a batch split and blame an innocent branch. CLM can detect one, as a test that fails and then passes on the same tree, and reports it as a defect in the test against `land-checks-stay-fast`, without guessing its cause. Retrying a failed half once before blaming still helps.
- **Rejected: speculative parallel checks** (Zuul-style: A, A+B and A+B+C checked at once). It needs N builds at the same time, and six parallel builds already filled the disk on 2026-11-07 (`builds-run-out-of-disk-mid-check`).

## What makes it hard in CLM

- **Separate processes.** Each `clm land` is its own process, usually a different build's. The lock holder has to take in the waiting lands (from the ordered queue in `code-lands-wait-in-arbitrary-order`), land them, and each waiter has to learn its outcome. A waiter that finds its branch on `main` should still print `landed <sha> <kind>`, since builds and `/clm:build` read that line.
- **Approval.** A branch that needs review lands only while its diff's fingerprint is unchanged (`land.approval`). Whether a batch keeps that depends on whether the fingerprint is of the branch's own changes or of the merged tree. Settle that first.
- **Conflicts inside a batch.** B can merge onto `main` cleanly but conflict with A. It leaves the batch and goes back to its owner for `clm rebase`.
- **Blaming a failure.** A flaky test fails the batch and the split can blame an innocent branch. Retrying a failed half once before blaming would help.
- **What each land does.** The gate per branch on a moving base, archiving complete changes, deleting resolved items, each branch's `landed` record and pull request: all must come out as they would landing one at a time.

## When it is worth it

Only when two or more lands that still run a check routinely wait at once. Measure that first: once `ledger/landed-recheck` is built, each `landed` record says whether the land ran nothing, its overlap or its own selection, and with the check times and the lock queue's waiting lines that shows how many lands still check, how deep they queue and how long each check is.

## Toolchains

Batching works on the project's check command, whatever it runs, so it needs nothing toolchain-specific. It saves the most where a check is long, which is often a compiled project's full build and test run. The split on failure re-runs the check on part of the batch; in a compiled language that run is incremental from the batch's own build, so CLM should not clean or move the build output between the batch and its halves, wherever that output lives.
