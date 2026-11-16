---
title: "A verify test fails when two runs list worktrees at once"
kind: bug
components: [verify]
---

`clm-verify::run two_runs_never_move_the_tree_under_each_other` failed in `clm land`'s check of `impl/plugin.hooks` (a branch that touches nothing of `verify`), and passed when run again alone. The panic was in `crates/clm-verify/tests/common/mod.rs:218`: `Git("git worktree list --porcelain -z: fatal: failed to read .git/worktrees/verify-tree/commondir: Success")`. One run lists worktrees while the other is still adding or removing `verify-tree`, so `git worktree list` reads a half-written entry. Expected: the test, or the code it drives, holds against that race, since a flaky test fails other components' lands (`land-checks-stay-fast`, deterministic tests).
