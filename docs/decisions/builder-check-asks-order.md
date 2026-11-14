---
title: "The builder check asks, per requirement, what unbuilt work it relies on, and a same-node reliance carries the other requirement's needs"
date: 2026-11-01
status: accepted
---

Strengthens the builder check, as `needs-alone-order`'s "would change if" asks. Refines `needs-alone-order`, `requirement-needs`, `dependency-work-is-a-requirement` and `blocked-by-backlog`.

**What happened.** After a change that moved work across several nodes landed, `/clm:implement` built the nodes `clm next` offered. Three builds found mid-build that a pending requirement relied on another requirement's unbuilt work that no `#### Needs` named, and stopped at it with a `question` item. Two did not notice and landed code that assumed the missing work. Part of the missing order was within one node, which a `Needs` can't name, so the builder check dropped it. The queue was right by the spec: nothing told it about the order.

**The gap.** The builder check asked an open question, "what would you need answered first", and asked for needs only in passing. A builder reading a change asks about what is unclear, not about what is clear but not yet built, so the order went unnamed. And `requirement-needs` makes a same-node need an error, since order within a node is the build's; that holds while both requirements are buildable, but not when the earlier one is blocked by another node's work: then the later one is unblocked, offered, and built ahead of it.

**Now.** The builder check asks a closed question for each requirement the change adds or modifies: which requirements not yet current does it rely on, on any node, among the change's own and those `clm pending` lists, answering `none` when there are none. Each named requirement of another node becomes a `#### Needs` line, as before; work no requirement states becomes one (`dependency-work-is-a-requirement`). A requirement relying on a pending requirement of its own node that has a `#### Needs` block carries those needs too, so it waits on the same work. A same-node reliance on a requirement with no needs needs nothing: one build builds both. `propose`'s skill and the prompts `clm draft` and `clm draft --fold` write ask it the same way.

**Why not more.**
- **Allowing same-node needs**, blocked while the needed requirement is blocked. It would keep transitive order without copying, but it changes the model, the pending report, the queue, the blockers view and the build prompt for a case that copying covers with no code (`yagni`). Would change if copied needs drift: a requirement lands ahead because one of its node's requirements it relies on gained a need its copy lacks.
- **A node-level wait** (`depends_on` ordering again) or holding every node a multi-node change touches until its other nodes land. `needs-alone-order`'s reasons hold: coarse, and it queues unrelated work.
- **A check that derives order from requirement text.** The heuristic `requirement-needs` and `dependency-work-is-a-requirement` rejected.
- **A build-side rule.** The run prompt already tells a build to stop and flag a requirement it can't build without faking another node's work (`workaround-flags`, `requirement-blocks`); a build that doesn't notice won't be helped by saying it again. The cheap place to name an order is the design.

**Would change if** builds still stop for, or land ahead of, a dependency that no `Needs` named after this: then a deterministic aid is what's next, such as listing a multi-node change's requirement pairs for the builder to answer.
