---
title: "A stale delta's report names what moved the element: main, or the earlier changes on the branch"
date: 2026-10-28
status: accepted
---

`clm sync` regenerates `specs/` by applying the changes a branch adds in ID order, each onto the tree the ones before it left (`regenerate-applied-specs`). On a fold, several changes on one branch modify the same Design, and each delta's base is `origin/main`'s hash, since each was drafted on its own branch; the second change to touch the Design is stale against the first, not against `main`. The report (`stale-delta-shows-main`) said `main changed design of config since <base>` all the same, and told the session to take the new base from `clm hashes config`, which reads the working tree's `specs/`, the branch's own apply of every change, and so gives the wrong hash too. On a fold of 19 changes a session read `main`'s history for an edit that sat in an earlier delta on its own branch.

So the regenerate keeps each *stage* of the tree, `origin/main`'s `specs/` and the tree after each change applied, and the report says who moved the element: `main` when no stage has the base, and each change applied after the base's stage that changed the element's hash. The base's text comes from its stage when one has it, which also lets the merge (`stale-delta-three-way-merge`) rebase a delta whose base an earlier change on the branch produced, where the history walk of `origin/main` found no text; the walk stays for a base no stage reaches. The merge line prints the hash to record instead of naming `clm hashes`, and the `Merge` commit names the same movers, so a delta rebased onto an earlier change reads `Merge <earlier> into <change>`.

Alternatives considered:
- **Keep `main changed` and add a note on folds.** The words are what a session acts on; a note after the fact doesn't stop it looking in `main`'s history first.
- **Walk the branch's own history for the base's text.** A base an earlier change produced sits in the stages the regenerate already computed; reading the branch's commits would find it too, but needs a parse of every `SPEC.md` version on the branch for text the run has in hand.
- **Require fold deltas to be rebased onto each other before the fold lands.** That is what the report asks for; this change makes it say onto what.
