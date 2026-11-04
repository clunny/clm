---
title: "Rebasing an applied spec branch regenerates specs/ from its deltas"
date: 2026-10-22
status: accepted
---

On a branch that adds an `applied` change, `specs/` is derived: it is `main`'s `specs/` with the change's deltas and assets applied. So when `clm sync` rebases such a branch, it doesn't merge `specs/`. During the rebase, conflicts in `specs/` keep `main`'s side. Afterwards, `specs/` is reset to `origin/main`'s and the change is applied again with `clm-apply`. That's `regenerate-dont-merge` applied to the apply output, and it resolves `reapply-after-rebase`.

The deltas, not `specs/`, are what the author wrote, so they're where any real conflict shows up: a delta whose base `main` has changed is stale, and `clm sync` stops and names it. Rebasing a stale delta needs judgment, so Claude does it with `clm hashes`, and `clm check` judges the result.

Only a branch that adds an `applied` change is regenerated. `main`'s side of a `specs/` conflict is kept on any branch that adds a change, whatever its status (`keep-main-specs-on-spec-branches`). Branches that add no change and touch `specs/`, such as a finish branch setting `status: active`, are rebased like any other file.

Regenerating can change the PR's diff when `main` changed nearby text, in which case `pinned-approval` drops a `HARD` PR's approval. That is intended: the human approved a diff, and they should see the new one.

Superseded on that point by `derived-specs-fingerprint`: regenerating onto a changed `specs/` leaves the fingerprint, and the approval, alone when the deltas and the other authored files are unchanged.
