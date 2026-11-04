---
title: "clm apply checks the graph a change leaves by the model's rules across nodes"
date: 2026-10-27
status: accepted
---

`clm apply` promises an applied change loads, but it checked each delta's node alone: the rules that join nodes, such as a required node not depending on an optional one, a `depends_on` naming a node, a `Needs` naming a requirement and a backlog item's `blocks`, were left to the next `clm check`. So a delta making `a` optional, while required `b` depends on it, applied cleanly and broke the tree.

Validation now builds the resulting graph in memory, parsing each written node with the model's node parser and dropping each removed node, and runs the model's graph check on it, the same function loading runs. It reports only what the change adds, comparing with the graph check on the current graph, so a graph that already has violations (as `clm check` may see) doesn't report them twice. This also ends `remove-node`'s exemption: removing a node something still names is refused, and the spec PR edits the dependants, in the same change for nodes and before `clm apply` for backlog items.

`reapply`, which `clm sync` runs to rebuild `specs/` on a newer `main`, keeps going without this rule: refusing there would block a sync that the load after it reports anyway.

Considered:
- Loading the resulting tree with `load_files`. The graph holds no backlog or decision source, no delta source and no asset contents, so `validate(graph, change)` can't rebuild the files without reading the disk or changing its signature, and loading the whole tree repeats every per-file rule apply has already run.
- Copying the cross-node rules into `apply`. It is the drift that let `optional: true` on the root through (`20261027-root-not-optional-on-apply`).
- Leaving it to `clm check`. The spec PR's own check catches it, but `apply` is all-or-nothing and promises a tree that loads.
