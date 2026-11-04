---
title: "clm sync rebases another live change's delta that this branch's apply made stale"
date: 2026-10-28
status: accepted
---

A spec branch that applies a change rewrites the sections its deltas modify. A `proposed` or `approved` change elsewhere, on `main` or beside it on the branch, whose delta modifies the same section recorded the old hash as its base, so on the branch that delta is stale (`hash-based-staleness`), and `clm check` fails the branch on it (`apply/stale-base`). Nothing could fix that but editing the other change's delta on this branch: its own author can't rebase it onto text that hasn't landed, and `clm sync` merged a stale delta three-way only for the changes the branch adds, against `main` (`stale-delta-three-way-merge`). On `spec/20261027-clm-serve-2` the session did that merge by hand with a script, twice.

So after regenerating, `clm sync` merges the stale deltas of every live `proposed` or `approved` change it didn't regenerate against the regenerated tree, with the same three-way merge, the same `merged` line and a commit of its own, `Merge <movers> into <change>`, naming the regenerated changes that moved the elements; what doesn't merge cleanly stops the run with the same report, naming those changes. An `applied` change's bases differ from the tree by nature and an archived change is history, so neither is read.

Editing another change's delta on this branch is what the hand merge did, and the gate already rates it by what moved (`modified-delta-tiers-by-element`): a Design merge is `AUTO`, a requirement `SOFT`, a Contract `HARD`. The merged text is in the branch's diff, `clm check` judges it, and a HARD branch's fingerprint changes, so a human approves the merged text again.

Alternatives considered:
- **Leave the other delta stale and let `clm check` ignore a stale base that this branch caused.** The branch would land with a delta that applies the old section over the new one when its change is applied, dropping this branch's text (`stale-delta-shows-main`).
- **Have the other change's author rebase after this branch lands.** Until then this branch fails `clm check` and can't land; the author would be rebasing onto text they can't see.
- **Merge only the deltas on `main`, not a `proposed` change the branch adds.** A proposed change beside an applied one on the same branch is stale the same way, and the fold flow (`triage-folds-hard-drafts`) puts several changes on one branch.
