---
status: applied
resolves: [a-claim-reads-live-at-the-instant-recompute-says-it-lapses]
---

## Intent

The time `clm_replica::Replica::recompute` names as the next change of the implementation view is the first second at which a live claim reads lapsed, so the view does change then (`backlog/a-claim-reads-live-at-the-instant-recompute-says-it-lapses`).

Today `recompute` returns a claim's tip time plus 4 hours, but `orchestrator`'s rule counts a claim lapsed only once its tip is *older than* the expiry, so at that instant the claim still reads live. In the `Replica` object the alarm fires then, recomputes, finds nothing changed, posts a notice that changes nothing on the page, and re-arms a second later, when a second recompute drops the claim and posts a second notice. `replica/batched-recompute` even states the second second as a special case ("one second after `now` for a claim that lapses at `now` itself, which is still live"), and `web.worker.replica/times-as-instants`' *a commit made two hours east* states 14:00 for a claim pushed at 10:00, the instant at which it still reads live.

## Approaches

- **`recompute` names the first second the claim reads lapsed (chosen).** One second past 4 hours after the tip's time. Only `replica/batched-recompute` changes, and the "one second after `now`" special case goes, since a live claim's first lapsed second is always after `now`. `web.worker.replica/times-as-instants`' scenario names 14:00:01. `alarm-batches`' *a claim lapses while a page is open* stays as written: its claim "lapses at 14:00", which now reads as the first second it reads lapsed, so the alarm fires then, the view no longer lists the claim and one notice is posted, which the current code fails.
- **The rule reads a claim lapsed at its expiry** (its tip at least 4 hours old). Every scenario keeps its 14:00, and `orchestrator`'s `lapses_at` would name a lapsed instant too, but it moves `orchestrator`'s rule (`next`'s "older than the expiry", its Design's "committed within the expiry", and the places, which share the call) and `check.health`'s own `lapsed-claim` count, making both core components pending for a one-second boundary no person sees. Rejected as the larger change (`yagni`): the boundary every command judges by stays where it is, and the one reader that needs the exact instant computes it from that rule.

## Outcome

- `replica/batched-recompute`: `recompute` returns, of the live claims, the earliest first second at which `orchestrator`'s rule reads one lapsed, one second past 4 hours after its tip's time. Its scenario *a claim lapses* now returns 14:00:01 at 11:00 and at 14:00, none at 14:00:01 and at 15:00, and 14:30:01 for `propose/x` pushed at 10:30.
- `replica`'s Contract: `recompute`'s line says "the first second a live claim reads lapsed" for "the next claim's expiry".
- `web.worker.replica/times-as-instants`: *a commit made two hours east* expects 14:00:01 UTC, and the requirement needs `replica/batched-recompute`, whose build gives that time.
- The fix's code is `crates/clm-replica/src/compute.rs`' `next_lapse`, giving `tip_time + 4 * 3600 + 1` and dropping its `.max(now + 1)`. `web.worker.replica`'s object arms its alarm from the time `recompute` returns, so its code needs no change, only its test of *a commit made two hours east*, and its test of *a claim lapses while a page is open*, which then shows the one notice.

## Edge cases and workarounds

- `orchestrator`'s `lapses_at` (the `flight` and implementation models' "lapses in") still names tip + 4 hours, a second before the claim reads lapsed. Shown to the hour, or as the app's "lapses in now", the second is not visible, so it stays.

## What the requirements no longer state

- `replica/batched-recompute`: "the earliest time one lapses, 4 hours after its tip's time, or one second after `now` for a claim that lapses at `now` itself, which is still live" becomes the first second the claim reads lapsed, one second past 4 hours after its tip's time.
- `replica/batched-recompute`, scenario *a claim lapses*: its outcome changes under its name, returning 14:00:01 where it returned 14:00 at 11:00, and 14:30:01 where it returned 14:30, with a recompute at 14:00:01 added that returns none.
- `web.worker.replica/times-as-instants`, scenario *a commit made two hours east*: its outcome changes under its name, each `recompute` returning 14:00:01 UTC where it returned 14:00 UTC.

## Open questions

None.
