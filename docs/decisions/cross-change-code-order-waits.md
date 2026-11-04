---
title: "A plan may make a task wait for another change's task"
date: 2026-10-23
status: superseded
superseded_by: node-reconciliation
---

Extends `code-order-waits` across changes. A task's code sometimes needs files or APIs that another live change's task writes, as `20261022-web-setup`'s `web` task needed `20261022-web-ui`'s. `cross-change-waits` only follows `depends_on`, and stays silent when the other change is ordered after or its work is on the same node, and `prefer_after` yields once the other change stalls. The plan's prose note was all that stood between `clm next` and a session that could only stop.

So a `waits_for` entry may be `<change>/<node>`. The task isn't ready until that change is archived or its task on the node is `done`, and nothing lifts the wait: not `--ignore-preference`, not priority. The `plan` session adds it, as for a same-change code-order wait, because it sees how the code is wired. `clm plan` keeps the entry while the named change is live and touches the node, and drops it once the change is archived.

The entry also orders the two changes for derived waits: a change whose plan names another's task counts as reaching it, as `after` and `prefer_after` do, so the other change's tasks never derive a wait back onto it and the two can't wait for each other. `clm check` rejects an entry naming a change that reaches this one through `after`, which could never start first, and any cycle among all live plans' entries read together.

Alternatives considered:
- Declaring the need in `change.md`. `code-order-waits` rejected this already: the order comes from code wiring the planner sees and the spec author often doesn't, and every edit would cost a human review.
- Making `cross-change-waits` also hold a task for another change's unbuilt task on its own node. It fixes the web-setup case and not the general one, a file another change writes on any node.
