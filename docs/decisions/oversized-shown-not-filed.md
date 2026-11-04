---
title: "An oversized node is shown, not filed: `clm sync` files no split item"
date: 2026-10-30
status: accepted
---

`oversized-is-a-state` made an over-budget node a visible state and had `clm sync` file `backlog/split-<node>.md` for it as a `question`, so a human would decide the split or raise the budget. The re-founding (`docs/the-clm-project.md`) found that loop feeding itself: more requirements pushed nodes over budget, which filed split items, which asked for more structure. `slim-nodes` now says splitting is the AI's business and needs no human, and the re-founding's freeze deleted every split item. But the filer brought them back, as human questions, on every build of an oversized node (`backlog/split-orchestrator.run`), and on a branch whose sync archived a change resolving one, it filed the item again in the same run, with new counts and another `kind`, so the code branch's net diff edited an existing item and the gate rated it `mixed`.

The design was wrong, not the filer's details: the state is already visible in `clm size`, `clm status`, the spec view and `clm health`, so the item only repeated it, aimed at the wrong person. So `clm sync` measures no size and files no split item. Over-budget is shown, not filed. The human accepted the eleven split items open on 2026-10-30 for now: "stop the growth" means no new nodes during the re-founding, so no node is split while it lasts. Splitting waits until the re-founding's mapping and pruning have shrunk the nodes, and the AI then decides splits along seams without a human. How it learns which nodes to split then is designed then, not now (YAGNI): `clm health`'s `oversized` section already lists them.

Rejected:
- **File the item as `kind: idea` for an AI session** (the first bug's suggestion). It still files during a freeze on new nodes, still creates churn on every build, and still repeats what `clm health` shows.
- **Keep filing but leave an existing item alone, and skip archive's deletion of an item it would refile** (the second bug's suggestion). It patches the refile and keeps the wrong design.
- **Let the gate accept a refiled split item**, as `code-fix-resolves-bug` accepts a resolved bug. A gate exception for a filer that shouldn't run.

A `size_budget` in a node's frontmatter still clears the state on purpose, as `node-size-budgets` says. `clm sync`'s reconsider items (`file-reconsider-item`) are left as they are: they are scored from friction and churn, not from the size, and no item in this change asked about them.
