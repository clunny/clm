---
status: done
---

## Intent

Lands waiting for a clone's land lock or check lock are ordered by the priority of the work they carry, as well as by cost. A `P1` land goes first, before even an already-checked, readers-only land. Below `P1`, the cheap lands go first, highest priority first, and then the rest, highest priority first. Arrival order breaks ties. The cap on cheap turns in a row rises from three to six and applies only below `P1`. A land's priority comes from `orchestrator`'s new `land_priority`: for an `impl/*` branch, the component's effective priority; for any branch, the `priority` of each change it adds and each backlog item it resolves; `P5` otherwise. Decision: `ordered-lock-queue` (amended).

## Problem

`20261108-ordered-land-queue` orders the queue by arrival, with a lane for already-checked lands. A `P1` build that finishes behind several `P5` builds still waits for every one of their full checks, although `priority-orders-nodes` has `clm next` start the urgent build first. The urgency is lost at the last step.

## Approaches

- **Derive the priority when the land queues** (chosen). It comes from what the branch and the base already hold: the component's effective priority, the changes the branch adds and the items it resolves. Nothing new is recorded, and every kind of branch gets a priority by one rule.
- **Record the priority on the branch or the place.** For example, `clm run` writes it into the build's place file. Rejected: spec, fix and small-change branches have no place, and a recorded value goes stale as `main` moves.
- **Priority above the cheap lane at every level.** Rejected by the human: below `P1`, a short check delays an urgent land little and spares the lands queued behind it.
- **A guaranteed turn for low-priority lands after some time.** Offered and not chosen: the human kept only the cap on cheap turns, raised to six. In the land queue, then, priority can hold a lower land back while higher ones keep arriving, unlike `priority-orders-nodes`' rule for readiness; the decision's amendment says why that is acceptable.

## Outcome

`land`'s `lock-queue` gains the priority, the turn order above and the cap of six, and needs `orchestrator/land-priority`, the new library call. `land`'s Contract and Design describe the order and the ticket name `<counter>-<lane>-p<n>`. Runs other than lands (`--check-only` and `verify`) wait at `P5`.

## Constraints

- A priority that can't be computed is `P5` with a warning. The order is never a reason to fail a land.
- `land_priority` reads git objects only, at the fetched base and the head.
- Exclusion stays the operating system's lock, as before.
