---
title: "Scratch repositories follow GIT_DIR to the clone"
kind: bug
components: [sync, orchestrator.run]
---

Two commands make a repository of their own in a temporary directory and run git in it through `clm-git`'s `git`, which inherits `GIT_DIR` and `GIT_WORK_TREE` from the environment: `replayed` in `crates/clm-sync/src/renames.rs` (`git init -q`, then `git add -A`) and the superpowers fetch in `crates/clm-run/src/lib.rs` (`git init -q`, `git fetch`, `git checkout`). Run with those variables set, as from a git hook or a work tree whose git directory is elsewhere, `git init` reinitializes the clone's git directory instead, and the commands after it stage, fetch or check out in the clone. They should run through `clm-git`'s `git_isolated` (`vcs/git-isolated`, change `20261108-keep-remote-fails-when`) once it is built. Found drafting that change for `keep_remote`, which had the same fault.
