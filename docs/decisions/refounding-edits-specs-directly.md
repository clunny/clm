---
title: "The re-founding edits the specs directly, without deltas or clm apply"
date: 2026-10-29
status: accepted
---

The re-founding (`docs/the-clm-project.md`, `changes/archive/20261029-refounding`) changes CLM's own information model: the review tiers go, two human layers come in, and every rule is traced or cut. CLM can't carry a change to its own rules through itself. Phase 0 showed it: the installed `clm` held `view` and `plugin` as `waiting` on `orchestrator`, whose requirements `Needs` blocked on `view`, so `clm claim` refused the very builds that would have removed the wait.

So, once and for this work only:
- The re-founding edits `specs/`, `ARCHITECTURE.md` and `CAPABILITIES.md` directly, with no deltas and no `clm apply`.
- Each spec edit goes in the same branch as the code it requires, and tests cite the new versions from `clm hashes`, as usual (`versioned-markers`).
- `20261029-refounding` holds the design, the plan, the freeze's sort, the cost report and the mapping, and no deltas. It is archived by hand as the last step before the merge back.

The merge back is one commit made by hand, not by `clm land`: `refound`'s diff mixes spec and code, which the gate blocks, and `main`'s history would then hold a first-parent commit no `landed` record names. Local `main` is first reset to `origin/main`, dropping the freeze note that was never pushed, and the merge commit is `git merge --no-ff refound` with a message ending in the trailer `Clm-By-Hand: refounding-edits-specs-directly`. `clm ledger verify` accepts a first-parent commit whose `Clm-By-Hand` trailer names a decision present in that commit's tree with `status: accepted` (`ledger/verify-by-hand-merge`), so this file is what lets it through, and `git log --grep Clm-By-Hand` lists every such commit.

Every other invariant holds: `proven-by-tests`, `no-dead-code`, and `refound` passing every check after every merge. Once `refound` merges back into `main`, changes go through deltas and `clm apply` again.

Rejected: carrying the re-founding through CLM as ordinary changes (each one would be gated by the tiers it removes, and the queue would order its builds by the rules it rewrites); one large delta per node (thousands of lines of deltas whose bases go stale as the phases edit the same nodes, for a review the human already gives the design).
