---
title: "A node is split by nesting: the part carved out becomes its child, and the product keeps its ID"
date: 2026-10-25
status: accepted
---

The model has always allowed nested nodes (`specs/<a>/<b>/SPEC.md` is node `a.b` with parent `a`), the spine gives a child its parent's Contract and Design in every build (`spine-scoped-context`), `apply` creates a child whose parent exists, and the `spec` view and the web graph draw a node inside its parent. Until this decision no node used it, and the method never said when to.

When `slim-nodes` calls for a split, the part carved out becomes a **child** of the node it came from, and the half that stays keeps the node's ID. `web` stays the product and gains `web.worker`; `orchestrator` stays the queue and gains `orchestrator.run`, `orchestrator.watch` and `orchestrator.checkpoint`; `plugin` stays the plugin and gains `plugin.hooks` and `plugin.panel`. Which half keeps the ID: the one other nodes and invariants already name (`web-optional` names `web`), or the one that is the product rather than a component of it.

Why nesting rather than siblings:
- The child reads its parent's Contract and Design for free, which is the context a component needs: the Worker's build sees what the product promises, `orchestrator.run`'s sees the queue model.
- Only the carved-out requirements move. A requirement moves by being removed from the parent and added, text unchanged, to the child; its markers go `obsolete` in the parent and it is `specd` in the child, so the child's build writes its tests and the parent's build removes the old ones. Splitting into two new siblings would have renamed everything.
- The graph reads as architecture: a product with its components, not a wider list.

How the builds divide the move. A split is sequenced like any change another component must follow, and neither build edits the other's files (`coupled-changes-sequence`). A child's build owns its crate: it creates it, writes the moved code and its tests in its own files, cites its requirements, and adds its own entry to the shared wiring (`crates/clm/src/main.rs`, `Cargo.toml`), which `path-ownership` lets every build do. The parent gets a requirement, needing the child's, whose build deletes the parent's copy of the code, with the tests and markers of the moved requirements, and reaches the code through the child. So for what moved the child always builds first, whatever `depends_on` says, and the parent's obsolete lines wait for it (`moved-markers-wait`). Children in one wave may both touch `main.rs` and `Cargo.toml`; `clm sync`'s rebase shows the conflict and the session resolves it. The parent's Contract lists the library calls the children get; a child's requirement that relies on one the parent hasn't built yet says so with `#### Needs` (`needs-alone-order`).

What nesting doesn't change: `paths` stay disjoint across all nodes, parent included; `depends_on` is separate from ownership, so a parent may depend on its child (`plugin` on `plugin.hooks`) or a child on its parent (`orchestrator.run` on `orchestrator`); a child has its own budget, and the parent's preamble rides in the child's context, so a parent's Contract and Design should stay short.

Rejected:
- **Flat siblings** (`watch`, `run`, `hooks`, `panel`, `web-app` at the top level). Every moved requirement and most of the unmoved ones would have changed ID, the node map would have doubled, and the components would have lost the parent's context they need.
- **Splitting by moving code between crates without new nodes.** It leaves the spec, which is what every build loads, exactly as long.
