---
title: "The dev-install rebuild follows origin/main through a reference-transaction hook"
date: 2026-11-02
status: accepted
---

`dogfood-from-main` kept `CLM_BIN` on `main` with a `post-merge` hook in the main worktree. But `main` moves through `clm land`, which pushes it from the worktree that lands (`land-locally`), and during `/clm:implement` every land comes from a build's worktree. The main worktree never merges, the hook never runs, and the session's `clm` stays at the commit it started on. That was accepted once (2026-10-30, `20261030-clm-run-hands-out`) as costing one stopped build now and then. It costs more: once a landed change used a new frontmatter key in a backlog item, the stale binary rejected the item, so the orchestrating session's `clm next` failed and every build's `clm land` failed `clm check` on its merge with `main`. One build read that as `main` being broken and filed a wrong defect, others landed with a branch-built `cargo run -p clm`, and the human rebuilt by hand mid-session. Any change that adds a key or a file format CLM reads does this (`config`'s `settings-before-use` is the same wait, for `clm.toml`).

The design was wrong in its trigger, not its aim: it watched the one place `main` rarely moves. Every way `main` moves on a clone moves `refs/remotes/origin/main`. A land's push updates it, and so does a fetch or a pull, a rebasing one included. Git runs the `reference-transaction` hook, from the git directory every worktree shares, on each ref update in any worktree. So `scripts/dev-install.sh` installs that hook instead, and the rebuild builds `origin/main`'s commit:

- **From its own source.** A `git clone --shared --no-checkout` in the common git directory, `clm-install-src`, checked out detached at the commit. It borrows the clone's objects, never touches a worktree anyone works in, and isn't a worktree of the clone, so nothing in CLM that lists worktrees sees it. `build.rs` reads the commit there, so `clm --version` names it.
- **Only when Rust changed.** The installed binary's `--version` names its commit (`dev-install-script`); a diff from there to `origin/main` over `crates/`, `Cargo.toml` and `Cargo.lock` decides.
- **One at a time, ending at the latest.** A lock directory holding the rebuild's process ID. A rebuild that finds it held by a running process leaves the work to that one, which reads `origin/main` again after each install, so a burst of lands ends with the binary at the last of them.

A build still never installs `clm`. The rebuild is the human's setup acting on what reached `main`, as the `post-merge` hook was, and it only moves the binary forward along `main`, which every build merges anyway. `cargo install` renames the new binary into place, so a `clm` that is running keeps its file, and `clm land` runs from a copy of itself (`land/land-binary-copy`).

Consequences:
- A session's `clm` reaches a landed Rust change once the background rebuild finishes, a few minutes; in that window the old binary still runs.
- Each rebuild compiles in the background while builds compile, on the same machine, only when Rust code changed.
- The hook runs on every ref update in the clone and exits at once unless the update is `committed` and names `refs/remotes/origin/main`.
- A pull that rebases now rebuilds too, which the `post-merge` hook didn't.
- A clone set up before this keeps its `post-merge` hook, whose `--after-merge` the script no longer knows, until the human runs `scripts/dev-install.sh` again, which deletes it and writes the new hook.

Alternatives considered:
- **Accept it, as before.** Rejected: the cost is the whole session once a landed change adds something CLM parses, and builds misdiagnose it.
- **`clm` compares its build commit with `origin/main` and says it's stale.** Product code serving no capability: an adopter's `clm` is a release with no relation to their `main` (`yagni`). It would also only name the problem, leaving the rebuild to the human.
- **Fast-forward the main worktree from the hook, so `post-merge` runs.** It changes files in a checkout a human or a session may be working in, from another session's push.
- **Build from a linked worktree kept for the purpose.** It shows in `git worktree list`, which `clm run`'s leftover cleanup and the orchestrator read.
- **`cargo install --git <clone> --rev <commit>`.** Cargo's own checkout of the source may not be a git checkout `build.rs` can read, so `clm --version` would lose its commit.
