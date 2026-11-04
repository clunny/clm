---
title: "An over-budget node is a visible state, not a failure and not a priority"
date: 2026-10-25
status: accepted
---

`slim-nodes` wants a node that outgrows its size budget (`node-size-budgets`) to be dealt with, by a split or by a budget set on purpose. `deterministic-enforcement` wants a program, not prose, to see that it is. The question was what the program should do.

A node is **oversized** when its spec or code count exceeds a budget its frontmatter doesn't set for that count, or exceeds a `size_budget` it does set. A `size_budget` that covers the count is the exception a human chose in a spec PR, next to the node it is about; it clears the state. One the node has outgrown doesn't.

What a program does with the state:
- `clm size` prints `oversized` on the node's line.
- `clm status` shows the cell `oversized` on the node's row and counts the oversized nodes in its summary; the `spec` view marks the node in its chart (`⚠ oversized`) and SVG (amber, `colour-meanings`).
- `clm health` lists it under `oversized`. Until 2026-10-30, `clm sync` also filed `backlog/split-<node>.md` as a `kind: question`; it files nothing now (`oversized-shown-not-filed`).

Nothing blocks on it. No check, gate or sync step fails because a node is oversized, and a build of an oversized node merges like any other. The rule the programs enforce is "every oversized node is visible", not "no node is oversized".

Rejected:
- **A `clm check` violation.** The build that pushes a node over can't fix it: the remedy is a spec PR, which the builder can't make. Its code PR would sit red on a number, the stuck-in-the-moment situation CLM is built to avoid. A variant, failing only once the node has nothing pending, keeps `main` green through a split but still blocks the build that lands over.
- **The item at P2 or P3.** The capped slots exist so the human triages competing work (`priority-caps`). An item a program files into a slot either takes one from the human or bypasses the cap, which makes the priorities drift. An oversized node isn't competing work; it is a state of the graph, and CLM already shows state without blocking (`waiting on you`, the queue states).
- **A high priority outside the caps.** The same drift by another name.
