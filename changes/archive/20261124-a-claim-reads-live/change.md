---
status: done
resolves: [a-claim-reads-live-at-the-instant-recompute-says-it-lapses, workaround-web.worker.replica-alarm-batches-1]
---

## Intent

The time `clm_replica::Replica::recompute` names as the next change of the implementation view becomes the first second at which a live claim reads lapsed, so the view does change then, and a claim lapsing while a page is open gets one notice (`backlog/a-claim-reads-live-at-the-instant-recompute-says-it-lapses`). This change is the first of two steps; the second, `20261124-a-replica-names-the-lapsed-second`, sits on the same branch, `proposed`, and is applied once this one is built (`contract-step-deferred`).

Today `recompute` returns a claim's tip time plus 4 hours, but `orchestrator`'s rule counts a claim lapsed only once its tip is *older than* the expiry, so at that instant the claim still reads live. In the `Replica` object the alarm fires then, recomputes, finds nothing changed, posts a notice that changes nothing on the page, and re-arms a second later, when a second recompute drops the claim and posts a second notice. `replica/batched-recompute` even states the second second as a special case ("one second after `now` for a claim that lapses at `now` itself, which is still live"), and `web.worker.replica/times-as-instants`' *a commit made two hours east* states 14:00 for a claim pushed at 10:00, the instant at which it still reads live. The build of `alarm-batches` flagged the same fault (`backlog/workaround-web.worker.replica-alarm-batches-1`): its test can't assert the one notice its scenario states.

## Approaches

- **`recompute` names the first second the claim reads lapsed (chosen).** One second past 4 hours after the tip's time. `replica/batched-recompute` changes, and the "one second after `now`" special case goes, since a live claim's first lapsed second is always after `now`. Two of `web.worker.replica`'s tests pin today's second (`tests/times.rs`' `FOURTEEN_UTC`, `tests/events.rs`' `fired.contains(&lapse)`), and `replica`'s build may not edit them (`path-ownership`), so the change goes in steps (`coupled-changes-sequence`, `expand-contract`). `replica` is required and `web.worker.replica` optional, so `batched-recompute` can't carry a `#### Needs` on the object's requirements (`requirement-needs`), and the replica's step is a second change, held by a deferred item until the first is built (`contract-step-deferred`).
- **The rule reads a claim lapsed at its expiry** (its tip at least 4 hours old). Every scenario keeps its 14:00, and `orchestrator`'s `lapses_at` would name a lapsed instant too, but it moves `orchestrator`'s rule (`next`'s "older than the expiry", its Design's "committed within the expiry", and the places, which share the call) and `check.health`'s own `lapsed-claim` count, making both core components pending for a one-second boundary no person sees, and the object's tests, which pin today's second, would still have to move first. Rejected as the larger change (`yagni`): the boundary every command judges by stays where it is, and the one reader that needs the exact instant computes it from that rule.

## Outcome

This change (step 1), `web.worker.replica` only, holds before and after the replica's fix:
- `web.worker.replica/times-as-instants`: *a commit made two hours east* expects the time a replica holding only a claim pushed at 10:00 UTC returns when recomputed at 11:00 UTC, so its test computes the expected time from `clm_replica` instead of pinning 14:00 (`tests/times.rs`' `FOURTEEN_UTC`). It needs only `ledger/record-times`, as before.
- `web.worker.replica/alarm-batches`: *a claim lapses while a page is open* runs the alarms to a second past the time `t` the last recompute named, and asks that an alarm fired at `t`, the claim is gone, and each notice since names no event, so its test (`tests/events.rs`) drops its fixed `fired.contains(&lapse)`. The requirement drops its need on `replica/batched-recompute`, which is current and which its text no longer waits on. The workaround flag on it is deleted, since its scenario no longer states the one notice its test couldn't prove.
- `backlog/the-replica-names-the-lapsed-second-once-the-object-reads-its-time.md` (`kind: deferred`, `components: [replica, web.worker.replica]`) holds step 2 until `alarm-batches` and `times-as-instants` are current at the versions this change gives them.

Step 2, change `20261124-a-replica-names-the-lapsed-second`, applied once that item is judged due:
- `replica/batched-recompute`: `recompute` returns, of the live claims, the earliest first second at which `orchestrator`'s rule reads one lapsed, one second past 4 hours after its tip's time. Its scenario *a claim lapses* returns 14:00:01 at 11:00 and at 14:00, none at 14:00:01 and at 15:00, and 14:30:01 for `propose/x` pushed at 10:30. `replica`'s Contract says "the first second a live claim reads lapsed" for "the next claim's expiry". The fix is `crates/clm-replica/src/compute.rs`' `next_lapse`, giving `tip_time + 4 * 3600 + 1` and dropping its `.max(now + 1)`.
- `web.worker.replica/one-notice-per-lapse` (added, needing `replica/batched-recompute`): a claim lapsing while pages are open and no event was applied gets one notice, at the first second it reads lapsed, and no alarm for it earlier; *a claim lapses once*. The object arms its alarm from the time `recompute` returns, so its code needs no change: its build writes the test and removes the comment in `tests/events.rs` naming the backlog item.

No capability test sees a difference at either step: nothing under `crates/clm/tests/capability/` or `web/e2e/` observes `recompute`'s time or the notices around a lapse.

## Edge cases and workarounds

- `orchestrator`'s `lapses_at` (the `flight` and implementation models' "lapses in") still names tip + 4 hours, a second before the claim reads lapsed. Shown to the hour, or as the app's "lapses in now", the second is not visible, so it stays.
- Between the steps, the object still posts the extra notice at a lapse; nothing states otherwise until step 2.

## What the requirements no longer state

- `web.worker.replica/times-as-instants`, scenario *a commit made two hours east*: its outcome changes under its name, each `recompute` returning the time a replica holding only a claim pushed at 10:00 UTC returns at 11:00 UTC (14:00:01 UTC once step 2 is built) where it returned 14:00 UTC.
- `web.worker.replica/alarm-batches`, scenario *a claim lapses while a page is open*: its outcome changes under its name, an alarm firing at the time the last recompute named where it fired at 14:00, and "one notice was posted" becoming "each notice posted names no event". The one notice moves to step 2's `web.worker.replica/one-notice-per-lapse`, *a claim lapses once*, stated at 14:00:01.
- Step 2 lists its own under the same heading in its `change.md`.

## Open questions

None.
