---
title: "The replica names the second before a claim reads lapsed until its fix is applied"
kind: deferred
components: [replica, web.worker.replica]
origin: 20261124-a-claim-reads-live
---

Condition: `web.worker.replica/alarm-batches` is current at `d6095ba0` and `web.worker.replica/times-as-instants` at `a75b0e25` (change `20261124-a-claim-reads-live`), so the object's tests take the lapse time from `recompute` and hold for either second.

Then apply change `20261124-a-replica-names-the-lapsed-second` (`clm apply`, rebasing its stale deltas on `clm hashes replica` first): `replica/batched-recompute` names the first second a live claim reads lapsed, one second past 4 hours after its tip's time, where today it names the last second the claim reads live; and `web.worker.replica/one-notice-per-lapse` states one notice per lapse while pages are open. Applied before the condition holds, `replica`'s build would break `crates/clm-web-replica/tests/times.rs` and `tests/events.rs`, which it may not edit (`path-ownership`).
