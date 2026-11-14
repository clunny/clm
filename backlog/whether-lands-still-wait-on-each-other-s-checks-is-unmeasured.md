---
title: "Whether lands still wait on each other's checks once none is thrown away isn't measured"
kind: question
components: [land]
origin: 20261113-land-starvation
blocks: [land/batched-lands, land/failed-batch, land/unrecorded-land-finished]
---

`20261113-land-starvation` is built in two steps. Step one (`land/push-turn`, and the new text of `land/land-target-lock`, `land/lock-queue` and `land/overlap-recheck`) keeps every check that passed, so no land runs its check twice because `main` moved. Step two (`land/batched-lands`, `land/failed-batch`, `land/unrecorded-land-finished`) lets the lands waiting for a check share one. Step two changes how every land of a clone reaches `main`, so it waits until step one has run here and its effect is measured.

**The question for the human:** step one is in and measured; build step two now?

**How to answer it.** Once step one is installed and build sessions on this clone hold at least 50 lands that ran a check, read their `landed` records. The busy share is the sum of their `check_seconds` over the wall time from the first to the last of them in each session.

- **Under one half:** a land waits less than half a check on average, about 90 seconds at today's check times. Batching saves little at this load. Build it anyway if more builds are to run at once (the wait is about 2.5 minutes at 60% and about 15 at 90%); otherwise remove the three step-two requirements in a change of their own, which also deletes this item.
- **One half or more:** lands still queue. Delete this item, and `land`'s next build builds step two.

Deleting this item at any time builds step two at `land`'s next build.

**When step two is built**, the guide's page `docs/guide/ci-and-deployment.md` must say that lands waiting on a clone share one check and what a failed one does, in a docs branch of its own, since the guide is a spec path.

On 2026-11-13, before step one, the checks that won summed to 1,426 seconds over 79 minutes, a busy share of 30%; the discarded checks roughly doubled it.
