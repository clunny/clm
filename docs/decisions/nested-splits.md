---
title: "A node is split by nesting: the part carved out becomes its child, and the product keeps its ID"
date: 2026-10-25
status: accepted
---

The model has always allowed nested nodes (`specs/<a>/<b>/SPEC.md` is node `a.b` with parent `a`), the spine gives a child its parent's Contract and Design in every build (`spine-scoped-context`), `apply` creates a child whose parent exists, and the `spec` view and the web graph draw a node inside its parent. Until this decision no node used it, and the method never said when to.

When `slim-nodes` calls for a split, the part carved out becomes a **child** of the node it came from, and the half that stays keeps the node's ID. `web` stays the product and gains `web.worker`; `orchestrator` stays the queue and gains `orchestrator.run`, `orchestrator.watch` and `orchestrator.checkpoint`; `plugin` stays the plugin and gains `plugin.hooks` and `plugin.panel`. Which half keeps the ID: the one other nodes and invariants already name (`web-optional` names `web`), or the one that is the product rather than a component of it.

Why nesting rather than siblings:
- The child reads its parent's Contract and Design for free, which is the context a component needs: the Worker's build sees what the product promises, `orchestrator.run`'s sees the queue model.
- Only the carved-out requirements move. A requirement moves by being removed from the parent and added, text unchanged, to the child; its markers go `obsolete` in the parent and `specd` in the child, and the two builds rewrite them. Splitting into two new siblings would have renamed everything.
- The graph reads as architecture: a product with its components, not a wider list.

How the builds divide the move. (For the moved requirements' markers, `moved-markers-wait` replaces this: the node a requirement moved to builds first, and the parent's obsolete lines wait for it.) Which side goes first follows `depends_on`: when the children depend on the parent (`orchestrator`), the parent builds first; when the parent depends on the children (`plugin`, `web`), the children do. The parent's build owns every marker of a moved requirement that sits outside the children's `paths` (the CLI's integration tests in `crates/clm/tests`): parent-first, it removes them and leaves the tests uncited, marking the moved code `clm:remove-by <change>` so the commands keep working; children-first, it renames whatever the children haven't re-cited. A child's build owns its crate: it creates or takes it over, moves the code in, deletes the parent's copy, rewires `crates/clm/src/main.rs`, and cites its requirements, re-citing the parent's uncited tests with its own IDs. Children in one wave may both touch `main.rs` and `Cargo.toml`; `clm sync`'s rebase shows the conflict and the session resolves it. The parent's Contract lists the library calls the children get, and the parent's build makes them `pub` before any child needs them.

The child's edits to the parent's files are `path-ownership`'s fifth exception, which the build prompt states (`orchestrator.run/split-move-fix`), so a builder or an audit reading the constraint alone sees the move allowed.

What nesting doesn't change: `paths` stay disjoint across all nodes, parent included; `depends_on` is separate from ownership, so a parent may depend on its child (`plugin` on `plugin.hooks`) or a child on its parent (`orchestrator.run` on `orchestrator`); a child has its own budget, and the parent's preamble rides in the child's context, so a parent's Contract and Design should stay short.

Rejected:
- **Flat siblings** (`watch`, `run`, `hooks`, `panel`, `web-app` at the top level). Every moved requirement and most of the unmoved ones would have changed ID, the node map would have doubled, and the components would have lost the parent's context they need.
- **Splitting by moving code between crates without new nodes.** It leaves the spec, which is what every build loads, exactly as long.

(Amended 2026-11-12, `20261112-components-own-their-tests`: the child's build no longer moves files out of the parent. It writes the moved code in its own files, and a requirement of the parent's, needing the child's, deletes the parent's copy and rewires it (`coupled-changes-sequence`).)
