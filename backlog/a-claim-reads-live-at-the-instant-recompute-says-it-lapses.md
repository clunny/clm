---
title: "A claim reads live at the instant recompute says it lapses"
kind: bug
components: [replica]
---

`clm_replica::Replica::recompute` returns, as the time the implementation view next changes, a live claim's tip time plus the expiry (`compute::next_lapse`: `tip_time + DEFAULT_EXPIRY_HOURS * 3600`). But `clm_orchestrator::claim_state` counts a claim lapsed only once `now - tip_time` is *more than* the expiry, so at that instant the claim still reads live, and the view changes one second later.

What it does in the `Replica` object (`web.worker.replica/alarm-batches`): the alarm fires at the time `recompute` gave, recomputes, finds the claim still live, posts a notice that changes nothing on the page, and `next_lapse` (which never returns less than `now + 1`) re-arms it a second later, when a second recompute drops the claim and posts a second notice. So *a claim lapses while a page is open* sees two notices, not one, and the view at 14:00 still lists the claim; `times-as-instants`' *a commit made two hours east* states 14:00 as the time the view next changes.

Expected: the instant `recompute` names is one at which the view does change: either `next_lapse` gives the first second the claim reads lapsed, or the rule reads a claim lapsed at its expiry, whichever the specs settle (`replica/batched-recompute`, `orchestrator`'s claim expiry, and the two scenarios above agree on 14:00).
