---
title: "A linked worktree of a clone with a separate git directory reads no clm.local.toml"
kind: bug
components: [config]
origin: 20261119-wave-1-by-the-rule
---

`config/local-settings` says the clone's `clm.local.toml` is read whichever worktree of the clone the root is in. In a clone made with `git init --separate-git-dir`, whose common git directory isn't named `.git`, the main worktree reads the file beside its root and a linked worktree reads none (`crates/clm-config/src/lib.rs:953`), confirmed by a run. Wave 1 of phase 5 states that a root in such a linked worktree has no local layer, which is what the code does; whether it should find the main worktree another way is open.
