---
title: "clm watch waits three polls before re-running a cancelled check"
date: 2026-10-24
status: superseded
superseded_by: land-locally
---

Refines `watch-reruns-cancelled`. CI's `concurrency` group has `cancel-in-progress`, so a label (`spec-approved`) or a push cancels the PR's in-flight run when it starts a new one. For a few seconds the rollup shows the cancelled entries and nothing running, because GitHub hasn't registered the new run yet. A watch already polling then re-ran the old run, which joined the same concurrency group as the genuine new run: one of the two cancelled the other, wasting a run, and when the re-run's attempt was the one cancelled the watch printed a spurious `failed <check> (cancelled)`. `watch-expects-head`'s grace for old failures didn't cover it, since the cancellation completed after the watch started.

So a cancelled latest entry counts as running on the first three polls on which that same entry (same run, same `startedAt`) is its check's latest, and is judged from the fourth, as `watch-expects-head` judges old failures. The re-run's own attempt gets the same grace, since a label can cancel it just the same. A new run registers well within that, and a newer entry of the check is judged as usual. A genuine cancellation, such as a queued `merge` job its concurrency group dropped, is re-run about 90 s later than before, which is cheap next to a PR that otherwise sits open.

Alternatives considered:
- Listing the workflow's runs (`gh run list`) before re-running, and skipping the re-run when a newer run exists. Exact only if the runs API sees the new run sooner than the rollup does, which isn't established, and it adds a second API shape, as `watch-expects-head` declined to.
- Both the grace and the run list: the most code for little more.
