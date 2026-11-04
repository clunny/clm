---
title: "Backlog items and changes have a priority, P1 to P5, with capped slots"
date: 2026-10-23
status: accepted
---

A backlog item or a change may set `priority: P1` to `P5`; without one it is `P5`. The higher levels have fixed slots, shared by backlog items and live changes together: one `P1`, two `P2`, three `P3`, four `P4`, and any number of `P5`. `clm check` enforces the caps (`check/priority-caps`), so raising one thing means demoting another, and a high priority stays meaningful. A change carries the priority of the items it resolves, so an item named in a live change's `resolves` isn't counted again. An item promoted with `from` is deleted, so its slot passes to the change.

Priority orders work. `clm backlog` lists items by priority, `propose` suggests the highest first, and `clm next` lists ready work by effective priority. It holds work back only where order already matters: between live changes that share a node. There a difference in priority counts as a `prefer_after` from the lower change to the higher, so `change-order` is satisfied without a hand-written entry. An explicit `after` or `prefer_after` between the two still wins.

Priority never lifts a cross-change wait (`cross-change-waits`): the wait stops a node being built from text that relies on another change's missing work, and urgency doesn't change that. Instead the awaited work inherits the waiting change's priority (`priority-inheritance`), as a scheduler avoids priority inversion. It's listed first, no preference holds it back, and when it is blocked, `clm next` says so, naming the urgent item waiting on it, so the human can see what to unblock.

Rejected:
- **Uncapped priorities.** They drift until most things are high and the field says nothing.
- **Separate caps for backlog and changes.** A promoted item would then free a backlog slot and take a change slot, so promoting would quietly raise the total. One pool keeps the count of urgent work fixed.
- **Holding back all lower-priority work.** `implement` runs every ready item in parallel, so holding back work that doesn't overlap would only slow it down.
- **Lifting cross-change waits for higher priority.** Either the builder is trusted to skip the lower change's text, which no program checks (`deterministic-enforcement`), or `context` rebuilds the node's spec without the lower change's deltas from git history, a much larger change.
