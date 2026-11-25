---
status: proposed
---

## Intent

Step 2 of `20261124-a-claim-reads-live`, which states the design: `clm_replica::Replica::recompute` names, as the time the implementation view next changes, the first second at which a live claim reads lapsed by `orchestrator`'s rule, one second past 4 hours after its tip's time, and the `Replica` object posts one notice for a lapse while pages are open, at that second.

It is applied once `web.worker.replica/alarm-batches` and `web.worker.replica/times-as-instants` are current at the versions `20261124-a-claim-reads-live` gives them, whose tests then hold for either second, so `replica`'s build lands on a green `main` (`contract-step-deferred`, `coupled-changes-sequence`): `replica` is required and `web.worker.replica` optional, so `batched-recompute` can't carry that wait as a `#### Needs`. `backlog/the-replica-names-the-lapsed-second-once-the-object-reads-its-time.md` holds it until then.

## What the requirements no longer state

- `replica/batched-recompute`: "the earliest time one lapses, 4 hours after its tip's time, or one second after `now` for a claim that lapses at `now` itself, which is still live" becomes the first second the claim reads lapsed, one second past 4 hours after its tip's time.
- `replica/batched-recompute`, scenario *a claim lapses*: its outcome changes under its name, returning 14:00:01 where it returned 14:00 at 11:00 and at 14:00, and 14:30:01 where it returned 14:30, with a recompute at 14:00:01 added that returns none.
