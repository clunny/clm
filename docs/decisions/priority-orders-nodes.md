---
title: "Priority orders nodes, flows down the graph to what urgent work needs, and never holds work back"
date: 2026-10-23
status: accepted
---

`priority-caps` gives backlog items and changes a capped `priority`, and at first let a difference in priority between two live changes sharing a node hold the lower one back. With building per node (`node-reconciliation`) there is no order between changes to set.

A pending requirement's priority is the highest among the live changes whose deltas add, modify or remove it, `P5` when none does. A node's own priority is the highest among its pending requirements that no item blocks. Its *effective* priority is the highest of its own and of the effective priorities of the nodes that wait on it, directly or through other nodes: urgent work pulls forward the dependencies it needs, as a scheduler avoids priority inversion. This replaces `priority-inheritance`, which did the same along the old item waits.

Priority only orders. `clm next` lists ready nodes by effective priority, then those with no pending requirement that waits on other work before those with one, then ID, a build session takes its node's pending requirements highest first, and the views sort by it. It never makes a node ready or holds one back: `implement` runs every ready node in parallel, so holding back unrelated work would only slow it down (`priority-caps` said the same).

Refines `priority-caps`, whose caps and `clm backlog` order stand.

**The tie-break at equal priority** (2026-11-19, `20261119-the-build-loop-launches`). A ready node with a pending requirement that waits on other work is built again once that work is current, and `clm next` prints only as many nodes as there are free places, so at equal priority it lists the nodes that wait last. Seeding pulls the waited-on work forward only across priorities; at equal priority the ID alone used to decide. It only orders, and never holds a node back (`needs-alone-order`).
