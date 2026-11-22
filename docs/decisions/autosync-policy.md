---
title: "Autosync pushes spec-work snapshots to autosync/<branch> at the end of each turn"
date: 2026-10-22
status: accepted
---

Spec work on a `spec/*` branch is snapshotted at the end of every Claude turn (a `Stop` hook) and force-pushed to `origin` as `autosync/<branch>`. The snapshot is `HEAD` plus the uncommitted spec paths, built with `git commit-tree` under a temporary index, so it touches neither the working tree, the index nor the branch. That answers the `autosync-policy` question.

- **When: the `Stop` hook**, not each edit or a timer. A turn is a natural unit of work, the hook needs no daemon, and a turn that changed nothing pushes nothing.
- **Where: a separate branch**, never the spec branch itself, a PR head or `main`. CI runs on pull requests, so pushing a half-finished edit to an open `SOFT` or `AUTO` spec PR's branch could merge it on passing checks. `autosync/<branch>` belongs to no PR, so it triggers nothing. The flow still pushes the real branch deliberately.
- **What: only spec paths**, and only on `spec/*` branches. Code is the implement flow's job, and its sessions push their own claim branches. Leaving out other files means an untracked secret or scratch file never leaves the machine.

Autosync gives backup and visibility. Convergence, which the question also raised, comes from `clm sync` (`regenerate-applied-specs`, `sync-owns-merge-drivers`): rebasing early and often does more to reduce merge pain than pushing often.

## The rule in full

`ARCHITECTURE.md` holds no entry for this; its rules serve the capability `work-concurrently` (`human-layer-entries-stay-short`). The rule in full:

Spec-work snapshots SHALL go only to `autosync/<branch>`, never to the spec branch itself, a pull request's head or `main`, so half-finished edits can't land.

**Scenario: end of turn**
- GIVEN a session editing specs on `spec/x`
- WHEN its turn ends
- THEN a snapshot is pushed to `autosync/spec/x`, and `spec/x` is unchanged

Amended by `20261121-wave-3-sync-by-the-rule`: when the spec paths hold nothing uncommitted the snapshot is `HEAD` itself, not a commit on top of it (`sync/autosync`). "A turn that changed nothing pushes nothing" holds because a snapshot whose tree is already `origin/<branch>`'s or `origin/autosync/<branch>`'s is never pushed: a clean branch whose committed `HEAD` neither ref holds is pushed as it is, which is the spec work the turn committed. Off a `spec/*` branch, on a detached `HEAD` and outside a repository it does nothing and says nothing, since the hook runs it every turn.
