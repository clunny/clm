---
title: "Each node has a size budget in tokens, and keeping a node big is decided in its spec"
date: 2026-10-24
status: accepted
---

`node-reconciliation` says a node too hot to keep up is a reason to split it, and `spine-scoped-context` puts a node's whole spec into every build session's context, beside the code the session reads and changes. Nothing measured either, so a node could grow until each build of it was slow and costly, with no one prompted to ask whether it should be two nodes.

Now each node has two budgets: *spec tokens*, its `SPEC.md`, and *code tokens*, the code and tests under its `paths` that a session would read. Both are estimated as bytes ÷ 4, deterministic and with no tokenizer. Only files git tracks count, and of those, binary files and files `.gitattributes` marks `linguist-generated` or `linguist-vendored` are left out, so the measure follows what a session actually reads, using GitHub's existing convention rather than a list of CLM's own (`reuse-first`). `clm size` reports them.

When a build synced a node over either budget, `clm sync` filed `backlog/split-<node>.md` on that node's branch, so the item lands with the node's PR, and a human sees it. Splitting the node is a design change that only a human decides on. The other way out is to raise the node's `size_budget` in its frontmatter. That edit is spec text, so it goes through a spec PR that a human reviews, and the decision sits next to the node it is about. A per-node override in `clm.toml` was rejected: `clm.toml` is a code path, so raising a limit there would merge on checks alone. Recording the decision in the backlog item was rejected too, since it would bury it where no reviewer looks, and editing an item makes a code PR mixed. Deleting the item without raising the budget is allowed, but the next build files it again. Since 2026-10-30 `clm sync` files no split item at all, and the oversized state is the record (`oversized-shown-not-filed`); the budget in the frontmatter stays the way to keep a node big on purpose.

The budgets gate nothing: no size fails `clm check`, `clm sync` or the gate, and a node over budget still builds and merges. Since 2026-10-25 an over-budget node is the visible *oversized* state instead of advice (`oversized-is-a-state`), and a test counts toward the nodes its markers cite rather than the node whose `paths` hold it (`tests-count-where-cited`); `slim-nodes` says what to do about it. The defaults, 12,000 spec tokens and 40,000 code tokens, are a starting point that a repository's `clm.toml` can change.
