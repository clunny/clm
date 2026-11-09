---
status: applied
resolves: [a-land-s-check-fails-once-on-a-clm-land-test-waiting-for-another]
---

## Intent

Deletes `backlog/a-land-s-check-fails-once-on-a-clm-land-test-waiting-for-another` as already fixed by `20261108-overlapping-lands-fails-under`.

That change found the cause of this failure: `clm::land held_through_the_gate_on_the_merge` (and `overlapping_lands`) failed when the waiting land, after `waiting for another land's check`, re-fetched `origin` and lost a remote-tracking ref race to the first land's ledger push and branch deletion, and `clm land` called that fetch failure offline. `land/offline-land` now retries a failed fetch and probes `origin` before saying it is offline (built in `impl/land`, merged into `main` at 2026-11-08 16:08, and refined by `fetch-retry-in-layer`). The item was filed at 16:17 the same day from a land that ran before that fix reached `main`.

The item's diagnosis, that the test is sensitive to other lands on the machine, doesn't hold: each test builds its own fixture clone with its own `.clm/land` lock, and the land it waits for is the test's own second land. Timing failures from a loaded host under concurrent land checks are a separate item, `backlog/timing-bound-tests-fail-when-several-builds-land-checks-run-at`.
