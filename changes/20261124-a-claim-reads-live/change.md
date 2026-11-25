---
status: applied
resolves: [a-claim-reads-live-at-the-instant-recompute-says-it-lapses]
---

## Intent

The time `clm_replica::Replica::recompute` names as the next change of the implementation view is the first second at which a live claim reads lapsed, so the view does change then (`backlog/a-claim-reads-live-at-the-instant-recompute-says-it-lapses`).

Today `recompute` returns a claim's tip time plus 4 hours, but `orchestrator`'s rule counts a claim lapsed only once its tip is *older than* the expiry, so at that instant the claim still reads live. In the `Replica` object the alarm fires then, recomputes, finds nothing changed, posts a notice that changes nothing on the page, and re-arms a second later, when a second recompute drops the claim and posts a second notice. `replica/batched-recompute` even states the second second as a special case ("one second after `now` for a claim that lapses at `now` itself, which is still live"), and `web.worker.replica/times-as-instants`' *a commit made two hours east* states 14:00 for a claim pushed at 10:00, the instant at which it still reads live.

## Approaches

- **`recompute` names the first second the claim reads lapsed (chosen).** One second past 4 hours after the tip's time. `replica/batched-recompute` changes, and the "one second after `now`" special case goes, since a live claim's first lapsed second is always after `now`. Two of `web.worker.replica`'s tests pin today's second (`tests/times.rs`' `FOURTEEN_UTC`, `tests/events.rs`' `fired.contains(&lapse)`), and `replica`'s build may not edit them (`path-ownership`), so the change goes in three steps (`coupled-changes-sequence`, `expand-contract`): first `web.worker.replica`'s `alarm-batches` and `times-as-instants` state the time as the one `recompute` names, which holds before and after the fix; then `replica/batched-recompute`, which needs both; then `web.worker.replica/one-notice-per-lapse`, which needs it and states the one notice the bug denies, split off `alarm-batches` since that scenario can't hold until the fix lands (`requirement-needs`: needs of different requirements of two components in opposite directions are not a cycle).
- **The rule reads a claim lapsed at its expiry** (its tip at least 4 hours old). Every scenario keeps its 14:00, and `orchestrator`'s `lapses_at` would name a lapsed instant too, but it moves `orchestrator`'s rule (`next`'s "older than the expiry", its Design's "committed within the expiry", and the places, which share the call) and `check.health`'s own `lapsed-claim` count, making both core components pending for a one-second boundary no person sees. Rejected as the larger change (`yagni`): the boundary every command judges by stays where it is, and the one reader that needs the exact instant computes it from that rule.

## Outcome

- `replica/batched-recompute`: `recompute` returns, of the live claims, the earliest first second at which `orchestrator`'s rule reads one lapsed, one second past 4 hours after its tip's time. Its scenario *a claim lapses* now returns 14:00:01 at 11:00 and at 14:00, none at 14:00:01 and at 15:00, and 14:30:01 for `propose/x` pushed at 10:30.
- `replica`'s Contract: `recompute`'s line says "the first second a live claim reads lapsed" for "the next claim's expiry".
- Step 1, `web.worker.replica/times-as-instants`: *a commit made two hours east* expects the time a replica holding only a claim pushed at 10:00 UTC returns when recomputed at 11:00 UTC, so its test computes the expected time from `clm_replica` instead of pinning 14:00 (`tests/times.rs`' `FOURTEEN_UTC`). It needs only `ledger/record-times`, as before.
- Step 1, `web.worker.replica/alarm-batches`: *a claim lapses while a page is open* fires an alarm at the time `t` the last recompute named and runs the alarms to a second past it, and asks only that each notice since names no event, so its test (`tests/events.rs`) drops its fixed `fired.contains(&lapse)`. The requirement drops its need on `replica/batched-recompute`, which would now be a cycle, and whose scenario no longer waits on anything.
- Step 2, `replica/batched-recompute`: needs both step-1 requirements, so `replica`'s build lands on a `main` whose `web.worker.replica` tests hold for either second. The fix is `crates/clm-replica/src/compute.rs`' `next_lapse`, giving `tip_time + 4 * 3600 + 1` and dropping its `.max(now + 1)`.
- Step 3, `web.worker.replica/one-notice-per-lapse` (added): a claim lapsing while pages are open and no event was applied gets one notice, at the first second it reads lapsed, and no alarm for it earlier; *a claim lapses once* (pushed at 10:00, read at 13:58: an alarm at 14:00:01 and none at 14:00, exactly one notice). It needs `replica/batched-recompute`. `web.worker.replica`'s object arms its alarm from the time `recompute` returns, so its code needs no change: the build writes the test, and removes the comment in `tests/events.rs` naming the backlog item.
- No capability test sees a difference: nothing under `crates/clm/tests/capability/` or `web/e2e/` observes `recompute`'s time or the notices around a lapse.

## Edge cases and workarounds

- `orchestrator`'s `lapses_at` (the `flight` and implementation models' "lapses in") still names tip + 4 hours, a second before the claim reads lapsed. Shown to the hour, or as the app's "lapses in now", the second is not visible, so it stays.

## What the requirements no longer state

- `replica/batched-recompute`: "the earliest time one lapses, 4 hours after its tip's time, or one second after `now` for a claim that lapses at `now` itself, which is still live" becomes the first second the claim reads lapsed, one second past 4 hours after its tip's time.
- `replica/batched-recompute`, scenario *a claim lapses*: its outcome changes under its name, returning 14:00:01 where it returned 14:00 at 11:00, and 14:30:01 where it returned 14:30, with a recompute at 14:00:01 added that returns none.
- `web.worker.replica/times-as-instants`, scenario *a commit made two hours east*: its outcome changes under its name, each `recompute` returning the time a replica holding only a claim pushed at 10:00 UTC returns at 11:00 UTC (14:00:01 UTC once `replica/batched-recompute` is built) where it returned 14:00 UTC.
- `web.worker.replica/alarm-batches`, scenario *a claim lapses while a page is open*: its outcome changes under its name, an alarm firing at the time the last recompute named where it fired at 14:00, and "one notice was posted" becoming "each notice posted names no event". The one notice moves to `web.worker.replica/one-notice-per-lapse`'s *a claim lapses once*, stated at 14:00:01.

## Open questions

None.
