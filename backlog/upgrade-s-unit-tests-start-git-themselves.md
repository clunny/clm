---
title: "upgrade's unit tests start git themselves"
kind: deferred
components: [upgrade]
blocks: [vcs/one-git-layer]
origin: 20261119-wave-1-by-the-rule
---

`crates/clm-upgrade/src/tests.rs` and `crates/clm-upgrade/src/commands_tests.rs` run `git` with `std::process::Command` (`git init`, `git add`, `git commit`, `git worktree add`). `vcs/one-git-layer` now reads every crate's `src/`, test modules included, and fails on a crate that starts `git` itself, so `vcs`'s build can't pass until these go through `clm-git`, in files that build may not edit.

The fix, under `clm claim upgrade --fix backlog/upgrade-s-unit-tests-start-git-themselves`: add `clm-git` to `clm-upgrade`'s `[dev-dependencies]` (`upgrade` depends on `vcs` since `20261119-wave-1-by-the-rule`), run the two `git init` calls through `clm_git::git`, and the helper `git` in `tests.rs` through `clm_git::run` with its two `GIT_CONFIG_` variables in `Options::env`. No test asserts anything else. It states no behaviour, so it has no requirement; it blocks `vcs/one-git-layer` so the queue holds `vcs`'s build of that requirement until it has landed.
