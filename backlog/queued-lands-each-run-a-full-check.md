---
title: "Lands waiting in the queue each run a full check of their own, one after another"
kind: idea
components: [land, land.approval, ledger, gate]
---

When N lands queue for a clone's check, each runs the full check on its own merge with `main`, one after another, so the last waits N checks. Most of those checks pass. Merge queues (GitHub's merge queue, bors) instead check one tree holding `main` and all the queued branches, land them all when it passes, and split the batch in halves when it fails, finding the failing branch in about log N checks. That saves up to N-1 checks with no more disk or compute at once, unlike checking speculative trees in parallel, which needs N builds at the same time.

## What makes it hard in CLM

- **Separate processes.** Each `clm land` is its own process, usually a different build's. The lock holder has to take in the waiting lands (from the ordered queue in `code-lands-wait-in-arbitrary-order`), land them, and each waiter has to learn its outcome. A waiter that finds its branch on `main` should still print `landed <sha> <kind>`, since builds and `/clm:build` read that line.
- **Approval.** A branch that needs review lands only while its diff's fingerprint is unchanged (`land.approval`). Whether a batch keeps that depends on whether the fingerprint is of the branch's own changes or of the merged tree. Settle that first.
- **Conflicts inside a batch.** B can merge onto `main` cleanly but conflict with A. It leaves the batch and goes back to its owner for `clm rebase`.
- **Blaming a failure.** A flaky test fails the batch and the split can blame an innocent branch. Retrying a failed half once before blaming would help.
- **What each land does.** The gate per branch on a moving base, archiving complete changes, deleting resolved items, each branch's `landed` record and pull request: all must come out as they would landing one at a time.

## When it is worth it

Only when two or more lands routinely wait at once. Measure that first: the ledger's land records and the phase times from `20261107-faster-land-check` should show how deep the queue gets and how long each check is.

## Toolchains

Batching works on the project's check command, whatever it runs, so it needs nothing toolchain-specific. It saves the most where a check is long, which is often a compiled project's full build and test run. The split on failure re-runs the check on part of the batch; in a compiled language that run is incremental from the batch's own build, so CLM should not clean or move the build output between the batch and its halves, wherever that output lives.
