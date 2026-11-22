---
status: applied
resolves: [a-branch-another-land-already-took-onto-main-lands-again-as-an]
---

## Intent

A `clm land` of a branch whose head `main` already holds lands nothing: no empty merge commit, no second `landed` record. When the branch's own merge on `main` is recorded it prints that land's `landed <sha> <kind>` line and exits 0, as a waiting land taken with others does; when it isn't, `unrecorded-land-finished` finishes it as today; and a head `main` holds any other way is refused with exit 1, naming the commit that brought it there. It asks this every time it reads `origin/main` to build or push a merge on, so a branch another land took while this one waited for a lock or ran its check is caught too. Resolves `backlog/a-branch-another-land-already-took-onto-main-lands-again-as-an`.

## Problem

Wave 3's `ci` build ran `clm land impl/ci` (head 492a886e). It waited for the check lock ("waiting for another land's check ... 1 ahead"), and while it waited `cbf3a863 Merge impl/ci into main` landed that same head with its own `landed` record. The waiting land then made `998a911f`, a second merge of 492a886e whose diff against its first parent is empty, pushed it, recorded it as a `spec` land (the gate reads an empty diff as one) and printed `landed 998a911f spec`.

The spec allows this. `land`'s Design, in **Finishing an unrecorded land**, says a head on `main`'s first-parent chain, or one whose merge a `landed` record names, "is not this rule's and lands as `land` states", which is a fresh merge. The code asks whether the holder of a batch landed the branch (`batch::landed_record`) only after a wait for the *land* lock (`woke` in `land.rs`), and the push turn's fetch of `main` treats a `main` that now holds the head as any moved `main`, rebuilding the merge on it (**A kept check**). A land waiting for the check lock or its push turn, or one whose code key was already checked and so took no land lock, is never asked.

## Approaches

1. **Ask only after the fetch at the start of each attempt** (drop the `woke` condition). Small, but misses the reported case: the other land landed the head while this one waited for the check lock, after its last start-of-attempt fetch, and the push turn's fetch then rebuilds the merge on the new `main`.
2. **Ask at every reading of `origin/main` a merge is built or pushed on** (chosen): the start of each attempt, every preparing again, and the push turn's fetch; and in a batch, against the commit each waiting land would merge onto. One question, `git merge-base --is-ancestor <head> <main>`, answered three ways, two of which already exist (`finish_found`, `unrecorded-land-finished`).
3. **Leave the empty merge and drop only the second record.** Rejected: `main` would still carry a commit no one meant to land, and its `landed` line would still name the wrong commit and kind.

## Outcome

`land` gains `already-on-main`; its Contract says what such a land prints and how it exits, and its Design gains **A head `main` already holds**, pointing **Finishing an unrecorded land**, **The push turn** and **Lands taken together** at it. Nothing else changes: `unrecorded-land-finished` keeps its own case, and a head that contains `main`, the usual case, is still merged with a merge commit (step 4).

Edge cases the change already knows:
- A land that finds the head on `main` in its push turn holds the land lock and the push turn; it releases both before going on, since finishing an unrecorded land takes the land lock itself. The `checked` record its check appended stands, harmlessly.
- A head `main` holds through another branch's land (a branch merged into the one that landed), or one on `main`'s first-parent chain itself (a branch at `main`'s tip), exits 1 rather than 0: no land of this branch exists to report, and landing it would only add an empty merge. A claim whose tip `main` holds is already released (`merged-claims-released`), so the refusal holds no claim.

## Constraints

- `redesign-before-workaround`: the design was wrong (it named a recorded head as one to land again), so the rule is restated, not patched in one place.
- `reuse-first`: the found case reuses `finish_found`, the path a waiting land taken with others already prints through.
- `git-is-truth`: whether `main` holds the head is git's ancestry, asked of the fetched `origin/main`.

## Open questions

- A land alone appends its `landed` record after its push and outside the land lock (Design step 7, **Places**), so a land that takes no land lock (checked code whose readers select nothing) can find the other land's merge on `main` before its record and finish it as unrecorded, appending a second record for the same commit when the pushing land's append follows. This change doesn't widen that window and leaves it as it stands; it is worth an item of its own if it shows.
