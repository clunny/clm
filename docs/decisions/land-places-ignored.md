---
title: "A land keeps its places under .clm/, which clm init ignores, not under target/"
date: 2026-10-30
status: accepted
---

`clm land` keeps everything it leaves in the main worktree under the `.clm/` of the CLM root there: it compiles into `.clm/land`, checks in the land tree `.clm/land-tree`, takes its lock on `.clm/land.lock` and keeps a failing check's log at `.clm/land-logs/<branch>.log`. This decides only where those places are: one land at a time per clone, one kept tree, the lock's span and the log's handling are `build-places`', `fixed-land-tree`'s and `land-check-log`'s.

**Why not under `target/`.** `target/` is Cargo's directory, and only a Rust project ignores it. With the places there, in a project in any other language nothing ignored them, so the first land left `target/land-tree` (a nested worktree, which `git add -A` records as an embedded repository), `target/land` and `target/land.lock` untracked in the work tree, and a later `add -A` committed them (`backlog/clm-land-leaves-untracked-target-in-a-non-rust-adopter-s-work`). `.clm/` is CLM's own directory for a clone's scratch, and `clm init` already makes the repository ignore it, asking git whether it does (`init-ignores-local-settings`), so the places are ignored whatever the language, with nothing new to ignore (`reuse-first`).

**Rejected.** `clm init` ignoring `target/`: it hides a directory CLM doesn't own in a project that may use the name, and every adopted repository would have to rerun `clm init` to get the line. `clm init` ignoring a `/target/land*` pattern: an entry tied to `clm land`'s naming in a directory CLM doesn't own, with the same rerun. A land writing its own ignore rule, in `.git/info/exclude` or a `.gitignore` inside `target/`: CLM editing a clone's git configuration, or ignoring a project's own `target/` files.

**Where in a subdirectory root.** The places are under the CLM root in the main worktree, `<main>/<prefix>/.clm/`, since that is the `.clm/` the root's `.gitignore` covers.

**Costs.** `cargo clean` doesn't clear the land's compiled code; removing `.clm/land` and `.clm/land-tree` does. A clone that landed while the places were under `target/` keeps its old `target/land`, `target/land-tree` and `target/land-logs/` until someone deletes them and runs `git worktree prune`.
