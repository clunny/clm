---
title: "A change is complete when its requirements are current, and clm sync archives it"
date: 2026-10-23
status: accepted
---

With nothing planned (`node-reconciliation`), there is no `finish` item to set a change `done`, archive it and delete the items it `resolves`. Whether a change is complete is derived: every requirement its deltas add or modify that the spec still holds is current, and no marker cites one it removed that the spec no longer holds (`versioned-markers`, `removed-requirements-build-out`). Both are judged against the spec as it is now, so a later change that removes what this one added, or adds back what it removed, never leaves it unable to complete. The archive move is bookkeeping that follows from that fact.

`clm sync`, on any `impl/*` branch, archives each complete change it finds at `HEAD`: sets `status: done`, moves it to `changes/archive/`, deletes the backlog items its `resolves` still names, and commits `Archive <id>`. The gate already rates all of that as code (`archive-move`, `backlog-additions`), so the build PR stays a code PR. Usually the branch that completes a change is the one that archives it, since completeness is tested on its own tree. When two PRs complete a change between them, the next `impl/*` branch to sync archives it, and `clm checkpoint` prints `complete <id>` so the `implement` skill can make a small archive PR when nothing else is running.

A complete, unarchived change is harmless in the meantime: the queue doesn't read change status, and `clm status` shows it as `complete`. So `clm check` doesn't fail on one, which keeps `main` green when two merges complete a change together.

Refines `task-deletes-fixed-items`: the build that fixes an item still deletes it in its own PR, and the archive step deletes what is left.

Rejected:
- **A deterministic `archive/<id>` queue item run without a model.** A fourth item kind in the queue, the views and the checkpoint, for a step `clm sync` can do as it regenerates.
- **Failing `clm check` on a complete, unarchived change.** It would break `main` after two PRs complete a change together, and the second PR's merge would be the one to fail.
- **Counting a requirement a later change removed as done.** It keeps a change's original size in `clm status`, but claims as built work that nobody built, and still needs a separate rule for a requirement added back.
- **CI archiving on merge.** CI would push content to `main` outside any PR, and bot merges don't run CI on `main`.
