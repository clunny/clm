---
title: "Builds and lands compile into shared cargo targets under the main worktree"
date: 2026-10-26
status: superseded
superseded_by: build-places
---

Superseded by `build-places`: a probe showed that a target directory shared between checkouts serves one checkout another's compiled code as fresh, so builds now compile into their own worktree's `target/`, a per-clone cap bounds how many run, and lands share `target/land` one at a time.

Builds compile into `<main>/target/build` and lands into `<main>/target/land`, where `<main>` is the repository's main worktree, the first one `git worktree list --porcelain` lists. `clm land` sets `CARGO_TARGET_DIR` itself, from whichever worktree it runs in. A build's cargo commands are run by the model, so `clm run`'s prompt gives the session the absolute path and tells it to set `CARGO_TARGET_DIR` on every command that runs cargo, and never on a `clm` command, so `clm land` keeps `target/land`. A `CARGO_TARGET_DIR` the human set wins in both.

**Why.** Each build worktree compiled the workspace into its own `target/`, 6-17 GB each; ten parallel builds filled the disk and stopped the session. `clm land` already meant to share `target/land`, but it took `<repo>` as the checkout it ran in, so a land run by a build in its worktree compiled into that worktree too. With one directory, dependencies compile once and concurrent builds serialise on cargo's lock, trading some parallel compile time for bounded disk.

**Two directories, not one.** A land checks the merged tree and is what other work waits on; sharing the builds' directory would queue it behind every build's compile.

**Why the prompt, not the environment.** `clm run` starts no process (`builds-in-subagents`), the Agent tool takes no environment, and each Bash call starts a fresh shell. A `.cargo/config.toml` written above the build worktrees would rely on Claude Code's worktree layout, which CLM doesn't (worktrees are found by branch, never by path), and one inside the worktree would be an untracked file in the build's diff or collide with a repository's own. The prompt rule can be missed; a missed command costs one more target directory, which the next `clm checkpoint`'s removal of the landed worktree reclaims.

**Growth.** A shared target keeps each worktree's workspace crates apart (cargo keys path packages by their path), so it grows with the builds it served until someone runs `cargo clean`; that is bounded by the number of distinct worktree paths, not by the number of builds running at once.
