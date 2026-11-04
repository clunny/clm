---
title: "A node is removed by a delta with remove: true that lists everything it removes"
date: 2026-10-25
status: accepted
---

A delta could create a node (`create: true`) but not remove one, so a node could neither be retired nor renamed: `specs/` reaches its text only through `clm apply` (`regenerate-dont-merge`), and `clm sync` regenerates a hand-deleted `SPEC.md`. Renaming the node `git` to `vcs`, which Claude Code's worktree isolation can't build under its old name, needs it.

A delta with `remove: true` removes its node. It records the bases of the node's frontmatter, Contract and Design, and lists every requirement under `REMOVED Requirements` and every asset under `REMOVED Assets`, each with its base, so a removal is as explicit as the text it deletes and is refused as stale when the node changed since it was written (`hash-based-staleness`). It may hold no other section. A node with children is removed only together with them, in the same change. A rename is a removal plus a `create: true` delta that adds the requirements word for word under the new ID.

The tests that cited the removed node's requirements are then `obsolete` (`removed-requirements-build-out`), but no build of that node will ever run to remove them. So an obsolete marker whose node no longer exists counts against the node whose `paths` hold its file, and against the root when none does: in a rename, the node that takes the code over re-cites its tests and the old markers go with the old files.

Considered:
- Marking the node `status: deprecated` and leaving it empty. `spec-is-current` says the spec describes only the system as it is.
- A `RENAMED Node` delta that moves the node and rewrites its requirement IDs in place. It handles only the rename, still has to rewrite every dependant's `depends_on`, and a removal is needed anyway to retire a node.
- Letting a spec PR delete `specs/<node>/` by hand. `clm sync` regenerates `specs/` from `main` and the deltas, so the deletion doesn't survive, and the gate would have nothing to rate but a missing file.
