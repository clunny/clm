---
title: "init gives the first explanation of graph shape; propose and implement point back at the same page"
date: 2026-10-27
status: accepted
---

A project's graph is first shaped when `/clm:init` drafts its node map, and boundaries are cheapest to fix then. So `init` owns the first explanation: before drafting the node map, new project or existing code, it tells the human in a few sentences what a node is for (the unit one build session holds, spec and code), that nodes split along seams by nesting, that plumbing several nodes copy becomes a shared node, and that a node over its budget is a visible state they decide, not an error, and points at `clm guide shaping-the-graph` for the rest. It also points at `clm guide using-clm` for how a change flows and where decisions go.

`propose` keeps its split rule (`propose-nests-splits`) and points at the same page, now through `clm guide`. `implement` pointed at it when its summary listed a `split-<node>` question, until `clm sync` stopped filing them (`oversized-shown-not-filed`); the oversized state now reaches a human through `clm status` and `clm health`.

Rejected:
- **`propose` owns it (first split).** By the first split the graph has already been drawn, possibly flat, by a session that was never told nodes nest, which is what happened in `20261025-size-budgets-and-stale-docs`.
- **Each skill carries the whole explanation.** Three copies of the same prose drift; the guide is the one source (`method-in-the-plugin`), and the skills say only what the step in front of them needs.
