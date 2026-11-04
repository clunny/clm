---
title: "Priority orders nodes, flows down the graph to what urgent work needs, and never holds work back"
date: 2026-10-23
status: accepted
---

`priority-caps` gave backlog items and changes a capped `priority`, and made a difference in priority between two live changes sharing a node act as a `prefer_after`. With building per node (`node-reconciliation`) there is no order between changes to set.

A pending requirement's priority is the highest among the live changes whose deltas add, modify or remove it, `P5` when none does. A node's own priority is the highest among its pending requirements that no item blocks. Its *effective* priority is the highest of its own and of the effective priorities of the nodes that wait on it, directly or through other nodes: urgent work pulls forward the dependencies it needs, as a scheduler avoids priority inversion. This replaces `priority-inheritance`, which did the same along the old item waits.

Priority only orders. `clm next` lists ready nodes by effective priority then ID, a build session takes its node's pending requirements highest first, and the views sort by it. It never makes a node ready or holds one back: `implement` runs every ready node in parallel, so holding back unrelated work would only slow it down (`priority-caps` said the same).

Refines `priority-caps`: the caps and `clm backlog`'s order stay; the sentence about ordering live changes that share a node no longer applies.
