---
title: "`sync/reapply`'s take-back on a branch that regenerates nothing reads as undoing a rename branch's rebased deltas"
kind: question
components: [sync]
choices:
  - "A rename branch takes nothing back"
  - "Say otherwise"
---

`sync/reapply` says that when `clm rebase` regenerated no change and a commit the branch holds since `origin/main` changes a delta file of a live `proposed` or `approved` change, it SHALL merge and take back the other live changes' deltas against `origin/main`'s `specs/`. A rename branch (`sync/regenerate-renames`) regenerates no change, and its `Rename` commit rebases the other live changes' deltas onto the renamed tree (`rename/other-deltas`), so read literally the rule takes those rebases back on every run and leaves each such delta stale.

The build of `sync` in wave 3 (`20261121-wave-3-sync-by-the-rule`) reads the rename path as standing "in place of the rebase and the re-apply", as `regenerate-renames` says, and skips the take-back there; `crates/clm-sync/tests/sync.rs::a_rename_keeps_the_deltas_it_rebased` proves it (without the skip it prints `took back contract of a in changes/o/deltas/a.md` and a stale-base violation). A spec change should state that a rename branch takes nothing back, or say otherwise, so that test cites a sentence that states what it proves (`code-fix-needs-a-written-rule`).
