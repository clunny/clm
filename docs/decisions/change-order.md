---
title: "Changes that touch the same node are ordered in change.md, hard (`after`) or soft (`prefer_after`)"
date: 2026-10-22
status: superseded
superseded_by: node-reconciliation
---

Within a change, `clm plan` orders tasks by `depends_on`. Across changes, nothing ordered them: every applied change's items entered `clm next` at once. Two changes building the same node could then race, or one could build against code another was about to change. That happened with `20261022-task-status`, `20261022-sync` and `20261022-web-ui`. Their order lived only in a PR body, against `deterministic-enforcement`.

- **`after` in `change.md`**: a list of change IDs. None of the change's items enter `clm next` until every change it names is archived. The spec PR's author sets it, and the reviewer sees it as part of the intent.
- **Hard and soft.** `after` is hard: use it when the change needs what the other one builds. `prefer_after` is soft: use it when building in that order is easier, for example fewer merge conflicts or a tool the other change provides, but either order works.
  - **What soft means.** A preference holds the change back only while the preferred change is making progress, meaning it has an item that is ready or claimed. If the preferred change stalls, for example blocked on a human, the preference yields, so a soft order never leaves work idle.
  - **Deterministic.** Whether a preference holds is decided from the queue computed without preferences, so preferences can't feed into each other.
  - **Overridable.** `clm claim --ignore-preference` and `clm run --ignore-preference` take an item that only a preference holds back, for when the human wants that change built first. Every other rule still holds. `clm next` doesn't change, since `implement` runs every item it lists.
  - **Not a sort order.** `implement` starts every ready item at once, so ordering `clm next`'s output would change nothing. Holding back while the other change progresses is what makes the preference real.
- **Required where it matters.** `clm check` flags a `proposed` or `approved` change that shares a node with another live change unless one reaches the other through `after` or `prefer_after`. It also flags cycles. Sharing a node is the signal that the author should decide an order, and whether it must hold. Usually a shared node only risks merge conflicts, so `prefer_after` is enough. Changes on disjoint nodes need no `after` and still run in parallel, so the order constrains only what it must.
- **Only new changes are checked.** Applied changes are already on `main`, as `stale-deltas` treats them, so turning the rule on never breaks `main`. The rule fires where the overlap is created: in the spec PR that adds the second change.
- **A change, not its tasks, is the unit.** Waiting for the whole earlier change is coarser than waiting for one of its tasks. But it needs no cross-change task IDs, it is easy to review, and a change's `finish` (archiving it) is the one moment its nodes are known to be stable.

Rejected:
- Ordering by apply date. It's implicit, and it would serialise changes that don't overlap.
- Cross-change `waits_for` on tasks, written in `plan.md`. It's finer-grained, but it couples plans, which `clm plan` regenerates, to other changes' task names. The waits a task does need on another change's tasks are derived by the queue instead (`cross-change-waits`).
