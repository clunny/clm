---
title: "Backlog items and changes have a priority, P1 to P5, with capped slots"
date: 2026-10-23
status: accepted
---

A backlog item or a change may set `priority: P1` to `P5`; without one it is `P5`. The higher levels have fixed slots, shared by backlog items and live changes together: one `P1`, two `P2`, three `P3`, four `P4`, and any number of `P5`. `clm check` enforces the caps (`check/priority-caps`), so raising one thing means demoting another, and a high priority stays meaningful. A change carries the priority of the items it resolves, so an item named in a live change's `resolves` isn't counted again. An item promoted with `from` is deleted, so its slot passes to the change.

Priority orders work. `clm backlog` lists items by priority, `propose` suggests the highest first, and `clm next` lists ready work by effective priority. It never makes work ready and never holds it back (`priority-orders-nodes`): building is per node (`node-reconciliation`), so nothing orders one live change against another, whether or not they share a node, and a pending requirement takes the highest priority among the live changes that add, modify or remove it. The one place priority decides who waits is the order of lands queued for a clone's locks (`ordered-lock-queue`).

Priority never lifts a wait: a requirement that a `Needs` or a backlog item blocks stays blocked however urgent it is, since the wait stops a node being built on work that is missing, and urgency doesn't change that. Instead the awaited work takes the waiting work's priority (effective priority, `priority-orders-nodes`), as a scheduler avoids priority inversion. It's listed first, and when it is itself blocked the queue names its blocker (`requirement-blocks`), so the human can see what to unblock.

Rejected:
- **Uncapped priorities.** They drift until most things are high and the field says nothing.
- **Separate caps for backlog and changes.** A promoted item would then free a backlog slot and take a change slot, so promoting would quietly raise the total. One pool keeps the count of urgent work fixed.
- **Holding back all lower-priority work.** `implement` runs every ready item in parallel, so holding back work that doesn't overlap would only slow it down.
