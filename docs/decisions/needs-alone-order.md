---
title: "Only Needs orders builds: depends_on no longer holds a node back, and the waiting state is gone"
date: 2026-10-29
status: accepted
---

Refines `node-reconciliation`, whose readiness waited for every node in `depends_on` to be settled, and `requirement-needs`, which added the finer wait. Part of the re-founding (`docs/the-clm-project.md`).

A node is now ready when it has an unblocked pending requirement, whatever the state of the nodes it depends on. `depends_on` keeps its other jobs: it says which nodes' code a node may use (`check`'s import checks), which nodes are optional (`optional-nodes`), and how the graph is drawn. It no longer orders builds or carries priority.

**Why.** The node-level wait was coarse: a node waited for every pending requirement of every dependency, even ones its own work had no use for, so unrelated work queued behind it and showed as `waiting` in every view. `Needs` names the exact requirement a requirement relies on, and the queue already honoured it; with both, the views showed two kinds of wait for one idea. A requirement that relies on another node's unbuilt work now says so with `#### Needs`, as `dependency-work-is-a-requirement` and `contract-removal-needs` already ask the proposing session to do.

**What follows.** The `waiting` state, its `on` field, the `flight` model's `waiting_on` and the `waiting { on }` blocker edge go from `orchestrator`, `view`, `web` and the panel; waits show only at requirement level, as `blocked` with `<node>/<slug>` blockers and `needs_pending`. Priority flows only along needs: a blocked requirement seeds its priority on the work it needs, and on through that work's own needs.

**Risk accepted.** Work that relied on `depends_on` ordering with no `Needs` written can now build in parallel with what it relies on. A build that finds its dependency's work missing stops at that requirement and records it (`friction-flags`, `requirement-blocks`), and the proposing session's builder check asks for the needs a change relies on.

**Would change if** builds often stop for a missing dependency that no `Needs` named; then the builder check, not a node-level wait, is what to strengthen.

Refined by `serving-rules-order-capabilities`: a capability, which can hold no `Needs`, also waits on the rules that serve it that are never built, or changed and themselves blocked.

Refined by `builder-check-asks-order`: the builder check asks per requirement what unbuilt work it relies on, as this decision's "would change if" asked.
