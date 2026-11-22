---
status: done
resolves: [a-branch-another-land-already-took-onto-main-lands-again-as-an]
---

## Intent

A `clm land` of a branch whose head `main` already holds lands nothing: no empty merge commit, no second `landed` record. When the branch's own merge on `main` is recorded it prints that land's `landed <sha> <kind>` line and exits 0, as a waiting land taken with others does; when it isn't, `unrecorded-land-finished` finishes it as today; and a head `main` holds any other way is refused with exit 1, naming the commit that brought it there. It asks this every time it reads `origin/main` to build or push a merge on, so a branch another land took while this one waited for a lock or ran its check is caught too. So that such a land finds the other land's merge already recorded, a land holds its push turn until its records are appended, and finishing an unrecorded land takes the push turn as well as the land lock. Resolves `backlog/a-branch-another-land-already-took-onto-main-lands-again-as-an`.

## Problem

Wave 3's `ci` build ran `clm land impl/ci` (head 492a886e). It waited for the check lock ("waiting for another land's check ... 1 ahead"), and while it waited `cbf3a863 Merge impl/ci into main` landed that same head with its own `landed` record. The waiting land then made `998a911f`, a second merge of 492a886e whose diff against its first parent is empty, pushed it, recorded it as a `spec` land (the gate reads an empty diff as one) and printed `landed 998a911f spec`.

The spec allows this. `land`'s Design, in **Finishing an unrecorded land**, says a head on `main`'s first-parent chain, or one whose merge a `landed` record names, "is not this rule's and lands as `land` states", which is a fresh merge. The code asks whether the holder of a batch landed the branch (`batch::landed_record`) only after a wait for the *land* lock (`woke` in `land.rs`), and the push turn's fetch of `main` treats a `main` that now holds the head as any moved `main`, rebuilding the merge on it (**A kept check**). A land waiting for the check lock or its push turn, or one whose code key was already checked and so took no land lock, is never asked.

## Approaches

1. **Ask only after the fetch at the start of each attempt** (drop the `woke` condition). Small, but misses the reported case: the other land landed the head while this one waited for the check lock, after its last start-of-attempt fetch, and the push turn's fetch then rebuilds the merge on the new `main`.
2. **Ask at every reading of `origin/main` a merge is built or pushed on** (chosen): the start of each attempt, every preparing again, and the push turn's fetch; and in a batch, against the commit each waiting land would merge onto. One question, `git merge-base --is-ancestor <head> <main>`, answered three ways, two of which already exist (`finish_found`, `unrecorded-land-finished`).
3. **Leave the empty merge and drop only the second record.** Rejected: `main` would still carry a commit no one meant to land, and its `landed` line would still name the wrong commit and kind.

## Outcome

`land` gains `already-on-main`; its Contract says what such a land prints and how it exits, and its Design gains **A head `main` already holds**, pointing step 4, **Finishing an unrecorded land**, **The push turn** and **Lands taken together** at it. A head that contains `main`, the usual case, is still merged with a merge commit (step 4), unless `main` also holds it.

Two requirements change so that the found case is the one a live land meets, not the unrecorded one:
- `push-turn`: every attempt holds the push turn until it has appended the records it appends after its push, not only until the push. So a land that fetches `main` inside its push turn, as the reported land did after its check, finds the other land's merge already recorded. The turn grows by the append's one ledger commit and push, seconds; a land alone already appends before it deletes and prints, and a batch already appends before it drops the land lock.
- `unrecorded-land-finished`: finishing holds the land lock and then the push turn, reusing those the land already holds and taking none twice, so it never finishes a land that is still appending inside its push turn, and a land that reaches it from its push turn or after a lock wait doesn't wait on itself.

Edge cases and choices the change already knows:
- A head `main` holds through another branch's land (a branch merged into the one that landed), one on `main`'s first-parent chain itself (a branch at `main`'s tip), or one whose merge is recorded under another branch name exits 1 and deletes nothing: no land of this branch exists to report, and landing it would only add an empty merge. A claim whose tip `main` holds is already released (`merged-claims-released`), so the refusal holds no claim.
- The question comes before the head is judged and the approval, hold and verify stop are read, so a branch already landed reports its land even when a rule of the newer `main` would refuse a new one.
- A branch the other land's cleanup removed from `origin` and the clone between two attempts is asked about at the head the earlier attempt resolved, rather than refused as a branch that doesn't exist.
- Where `main` merges the head more than once (as the reported incident left it), any `landed` record of the branch on one of those merges is the found case.
- The found case appends this machine's `effort` records for the branch, as any land does: an `effort` record's `id` is its session and branch, so one the earlier land already wrote is superseded, not doubled (`effort-accounted`).

## Constraints

- `redesign-before-workaround`: the design was wrong (it named a recorded head as one to land again, and let a land find a merge before its record), so the rules are restated, not patched in one place.
- `reuse-first`: the found case reuses `finish_found`, the path a waiting land taken with others already prints through, and the push turn the lands already take.
- `git-is-truth`: whether `main` holds the head is git's ancestry, asked of the fetched `origin/main`.
- `effort-accounted`: the found case still records this machine's effort for the branch.

## Open questions

- The architecture audit asked whether the guide (`docs/guide/how-it-works.md`) or the build skill should say that "already on `main`" means the work is landed and there is nothing to do. Left out: the refusal names the commit that holds the branch, and the found case, the one a session meets after its own or a batch's land, exits 0 with the usual `landed` line, which the skills already read.
