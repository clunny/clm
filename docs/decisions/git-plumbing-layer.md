---
title: "One node shells out to git and gh; everything else calls it"
date: 2026-10-25
status: accepted
---

Six crates each carried their own `git()` wrapper (`gate`, `check`, `ci`, `init`, `sync`, `orchestrator`), four their own `gh()`, and `is_ancestor` existed twice. Each copy had its own error shape and its own idea of how to find the command. `reuse-first` and `slim-nodes` both say the same thing about that: plumbing several nodes copy becomes a shared node.

Node `git` (crate `clm-git`) is that layer: `git`, `gh`, `fetch`, `is_ancestor`, `current_branch`, `worktrees`, `extract_tree`, one error type, both commands resolved on `PATH` so tests keep faking `gh` with a script. A test in the crate scans the workspace and fails naming any other crate that runs `git` or `gh` itself, so the layer stays the only one by a program, not by review (`deterministic-enforcement`). The crate depends on nothing else in the workspace, so every node can depend on it, and it never enters a wasm build.

The cost: one more dependency edge on most nodes, and a `git` build settles before its dependants are ready (`node-reconciliation`). The layer changes rarely, so that wait is seldom felt. The wrappers it replaces go in the dependants' next builds (`no-dead-code`, `expand-contract`).

Rejected:
- **A git library (`gix`, `git2`).** CLM's gate and CI already use the `git` CLI (`gate`'s Design), the fake-`gh` test convention relies on `PATH`, and a library would add a large dependency to replace a few dozen lines of `Command`.
- **Leaving the copies.** Six places to fix a quoting or error-handling bug, and the budget pressure on `orchestrator` would have been met by moving `repo.rs` around rather than by removing the duplication.
