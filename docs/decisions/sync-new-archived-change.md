---
title: "`clm sync` regenerates a change its branch archived, but never one `main` applied"
date: 2026-10-27
status: accepted
---

A change with nothing to build is archived on its own spec branch right after `clm apply` (`archive-in-spec-pr`), so its `change.md` is under `changes/archive/<id>/`. `clm sync` regenerated `specs/` only for a live `applied` change the branch adds, so on such a branch a rebase that met `main`'s edits to the same `SPEC.md` stopped on a conflict under `specs/` for the author to resolve by hand, which is what `regenerate-dont-merge` exists to avoid. Sync now also counts a *new archived change*, re-applies its deltas when its status is `done`, and leaves it archived.

Sync's *new archived change* is narrower than the gate's (`gate/change-tiers`): `changes/archive/<id>/change.md` is new on the branch (absent at `origin/main` and at the merge base), and `changes/<id>/change.md` is absent at both as well. The gate also counts a live change on `main` that a branch moves into the archive with more than `status:` lines edited, since its job is to rate the files a reviewer must read. Sync's job is to rebuild `specs/`, and `main`'s `specs/` already holds the deltas of any change that was live there, so re-applying them would fail on stale bases. Testing the paths alone, rather than diffing the move as the gate does, gives the same answer for every branch that can occur and needs no rename detection. An archived change's `frontmatter_keys` no longer count (`model/valid-frontmatter`), so one whose delta adds a node frontmatter key fails both the re-apply and `clm check`; such a change has something to build anyway and isn't archived on its spec branch.

Alternatives considered:
- Reusing the gate's definition verbatim. It would re-apply a change `main` already applied whenever a branch moves it with an edit, reporting stale deltas for a change the branch didn't write.
- Leaving archiving until after the last sync. `propose` can't know that a later `clm sync`, before `clm land`, won't be needed, and a branch already archived would still meet the conflict.
- Treating an archived change of any status as one to regenerate. An `abandoned` change's deltas were never meant to reach `specs/`; only `done` matches what `archive-in-spec-pr` produces, as only `applied` does for a live change.
