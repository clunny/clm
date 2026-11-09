---
status: done
resolves: [workaround-sync.regenerate-regenerated-tree-1]
---

## Intent

`sync.regenerate`'s rules state the crate's own functions (`regenerated`, `Tree::other_deltas`, `git` and `run`) instead of what `clm rebase` prints, so its tests call that code directly. They no longer drive a copy of `clm rebase`'s steps, which is the workaround `backlog/workaround-sync.regenerate-regenerated-tree-1` records. The `clm rebase` behaviour stays stated where its code is, in `sync`: `reapply` gains the step that merges the other changes' deltas, and `git-through-layer` gains the rule that every `git` command `clm rebase` runs goes through the retrying wrapper. `other-delta-merge` folds into `stale-delta-merge`, since both state one merge, run at two points.

## Problem

`stale-delta-diff`, `stale-delta-merge`, `other-delta-merge` and `retry-index-lock` say "`clm rebase` SHALL", but `clm rebase` is `clm-sync`'s, and `sync` depends on `sync.regenerate`, so the crate's tests can't call it. They drive a stand-in written in `crates/clm-regenerate/tests/regenerate.rs` (fetch, rebase, regenerate, mirror, other deltas, check). That copy can drift from `clm-sync`'s real sequence without any test noticing, and the stand-in proves neither the crate's API nor `clm-sync`'s wiring. `clm health` scores `regenerated-tree` 3 (one workaround, one rewrite, many scenarios), the project's only reconsider candidate.

## Approaches

1. **Reword the rules to the crate's API** (chosen). Each rule states what `regenerated`, `Tree::other_deltas`, `git` and `run` return and commit. The tests call them on a work clone of a bare `origin`, and the stand-in goes. `sync/reapply` and `sync/git-through-layer` state the `clm rebase`-level wiring the stand-in imitated, with the scenarios that left `sync.regenerate` (another change's delta merged or left stale, a branch that regenerates nothing, the lock met at the `Re-apply` commit). This puts each rule where its code is, which is what `sync-regenerate-split` cut the component for.
2. **Move the four rules to `sync` and leave the code in `clm-regenerate`.** `sync-regenerate-split` rejected this: a fix to them would be built on `impl/sync` in another component's `paths`.
3. **Accept the stand-in and delete the workaround.** It costs nothing now, but the copy keeps the drift risk and proves nothing `sync`'s tests don't.

Merging `other-delta-merge` into `stale-delta-merge`: both rules were the same three-way merge, one for the changes `ids` names, and one for the other live changes after the mirror. One rule with a clause for each caller states the merge once.

## Outcome

- `sync.regenerate`: a Contract listing the crate's API; a Design without the paragraphs that restated the rules or narrated the split; `regenerated-tree`, `stale-delta-diff`, `stale-delta-merge` and `retry-index-lock` stated on the API; `other-delta-merge` removed. Its build rewrites `crates/clm-regenerate/tests/regenerate.rs` on the API, deletes the stand-in (`sync`, `mirror`, `Report`), moves the `other-delta-merge` markers to `stale-delta-merge`, and rewords the code comments that name `other-delta-merge`. This branch deletes the workaround item.
- `sync`: `reapply` and `git-through-layer` modified as above, and the Contract and Design name `stale-delta-merge` where they named `other-delta-merge`. Its build adds the new scenarios' tests to `crates/clm-sync/tests/` and extends the scan.

## Constraints

- No behaviour of `clm rebase` changes. Each scenario that left `sync.regenerate` is restated on the API, moved to `sync`, or dropped as a duplicate of one kept: `stale-delta-diff`'s "requirement changed on main" ("main changed a line the delta changed", for a requirement) and "serves edited on main" (`stale-delta-merge`'s); `stale-delta-merge`'s "section gone from main" and "removed requirement" (now one, "element removed on one side"); `other-delta-merge`'s "proposed change the branch adds" (`Tree::other_deltas` reads `HEAD`, so a change on the branch and one on `main` are the same case); and `retry-index-lock`'s "lock kept" and "other failure not retried" at `clm rebase`'s level, where they are `git`'s and `run`'s, and `clm rebase`'s exit 2 and `rebase failed` are `sync/outcome-line`'s.
- `sync.regenerate` still can't depend on `clm-sync` (`check/undeclared-imports`, `splits-import-one-way`).
