---
title: "The plan session may add waits between a change's tasks that the code needs"
date: 2026-10-22
status: superseded
superseded_by: node-reconciliation
---

Refines `generated-plans`. Two tasks of one change sometimes have to land in an order that `depends_on` doesn't record: one node's code wires in something another node's task adds, as when `20261022-no-dead-code`'s `check` task wired the `clm` binary to a setting its `config` task added. `depends_on` describes the architecture, and adding an edge only to order one change's work would misstate it. So the `plan` session may add the other task's node to a task's `waits_for` in `plan.md`, and says why in the notes.

`check/plan-matches-change` keeps checking what it can: every derived wait is present, each extra entry names another task of the plan, and the waits form no cycle. `clm plan` keeps an extra entry when it re-plans, while the task it names is still in the plan, and drops one that would close a cycle with the derived waits, since the node graph wins. A code-order wait is a plan field like the others, so it is reviewed as code (`gate/plan-is-code`), not as intent.

Alternatives considered:
- Declaring the order in the spec PR, in `change.md`. The order comes from how the code is wired, which the planner sees and the spec author often doesn't, and every such edit would cost a human review.
- Leaving it to notes, as before. Nothing stops `clm next` offering both tasks at once, and the later session has to merge `main` mid-task.
