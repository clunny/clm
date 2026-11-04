---
title: "A change's progress follows its requirements through later moves and renames"
date: 2026-10-27
status: accepted
---

An `applied` change counts only the requirements the spec still holds, so one a later change removed doesn't hold it back (`archive-on-sync`). A rename is a removal plus a `create: true` delta that adds the same requirements under the new node (`node-removal`), so the same rule also dropped requirements that only moved: when `20261025-worktree-safe-node-ids` renamed `git` to `vcs`, the applied changes on `git` read as complete (0 of 0 counted), `clm sync` archived them on unrelated `impl/*` branches and deleted the backlog items they resolve, though `vcs/extract-paths`, `vcs/extract-as-checkout` and `vcs/raw-runs` were still `specd`. The rename's own builder answer called this intended, because the rename change tracks the moved work. It does track the requirements, but not the items the earlier changes resolve, and the earlier changes disappear from `clm status` before their work is built.

So a change's requirement the spec no longer holds is followed through every *move*: a change that has applied (`applied`, `implementing` or `done`, inside or outside the archive) and whose deltas remove `<a>/<slug>` and add `<b>/<slug>` to another node moves that requirement. The change counts the requirement it moved to, through chains of moves and into every node of a split. Only a requirement no move takes anywhere the spec holds is left out as removed.

Moves are not ordered against the change they're followed for: the rename that prompted this applied after the changes it stranded. The cost is a rare false positive, a node recreated under an old ID that re-adds a slug an earlier rename moved away and is then removed again, which would follow the old move.

Considered:
- Counting every requirement the spec doesn't hold as not done, as the bug report first suggested. A requirement a later change really removed would then hold its change live forever, which the rule exists to prevent.
- Leaving it as designed, and having the rename change resolve the earlier changes' items too. Each rename would have to copy them by hand, and the earlier changes would still drop out of `clm status` early.
- Requiring a move's added text to match the removed text word for word. A move that also rewords is still the same work, and the hash comparison adds nothing the counts need.
- A `moved_from` key on the new requirement. The removal and the addition in one change already say it, with nothing new to write or check.
