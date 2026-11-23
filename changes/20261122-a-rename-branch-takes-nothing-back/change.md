---
status: applied
resolves: [reapply-s-take-back-reads-as-undoing-a-rename-branch-s-rebased-deltas, reconsider-sync-reapply, workaround-sync-reapply-1]
---

## Intent

A rename branch takes nothing back: `sync/reapply` and `sync`'s Design now say that on a branch `sync/regenerate-renames` handles, `clm rebase` merges and takes back no other live change's delta, since the `Rename` commit rebased those deltas onto the renamed tree itself (`rename/other-deltas`). The human agreed it on 2026-11-22 in the wave 4 session, answering `reapply-s-take-back-reads-as-undoing-a-rename-branch-s-rebased-deltas` with "A rename branch takes nothing back".

## Problem

`sync/reapply` says that when `clm rebase` regenerated no change and a commit since `origin/main` changes a delta file of a live `proposed` or `approved` change, it SHALL merge and take back the other live changes' deltas against `origin/main`'s `specs/`. A rename branch regenerates no change, and its `Rename` commit deliberately changes those delta files, so the rule read literally takes the rebases back on every run and leaves each such delta stale. Wave 3's build of `sync` already skips the take-back on the rename path, flagged it as `workaround-sync-reapply-1`, and `crates/clm-sync/tests/sync.rs::a_rename_keeps_the_deltas_it_rebased` proves the skip; no sentence stated it (`code-fix-needs-a-written-rule`).

## Outcome

- `sync/reapply` gains the sentence and one scenario, *a rename keeps the deltas it rebased*, which is what that test shows; every existing scenario keeps its name and text.
- `sync`'s Design, step 4, says the same where it describes the run on a branch that regenerates no change.
- The sentence sits in `reapply`, not `regenerate-renames`, because the code's skip is a condition of the take-back itself: `clm-sync`'s `regenerate` returns early when `renamed` is set, beside its check that a commit moved a delta file.
- Nothing is cut. `sync`'s next build re-cites the test at `reapply`'s new version; the flag and the reconsider item go with this change.
