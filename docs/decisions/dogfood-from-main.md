---
title: "This repo's sessions run main's clm and the checkout's plugin, not the release"
date: 2026-10-23
status: accepted
---

CLM is built with CLM, and its sessions should use the CLM being built, not the last release. A release lags `main` by dozens of PRs, and the plugin's skills only reach a session through a release, since Claude Code caches a GitHub-sourced plugin by `plugin.json`'s version. So a clone of this repository that a human has set up with `scripts/dev-install.sh` (`dev-install-script`) runs:
- **The binary from `main`.** The main worktree's `.claude/settings.local.json` sets `CLM_BIN` to `clm-dev/bin/clm` under the clone's common git directory, which the plugin's `bin/clm` shim runs (`dev-install-scoped`). A `reference-transaction` hook rebuilds it in the background whenever `origin/main` moves and the move changes `crates/`, `Cargo.toml` or `Cargo.lock` (`rebuild-on-origin-main`). Every land's push moves `origin/main`, so the binary follows lands made from build worktrees, which never merge in the main checkout.
- **The plugin from the checkout.** A marketplace named `clm-dev`, whose root is the main worktree, is added through a manifest file under `.clm/` (`dev-marketplace-in-place`), and the settings file enables `<name>@clm-dev` and turns off `<name>@clm`. Claude Code loads the plugins from the main worktree in place, with no cache.

The setup stays inside the clone: the `clm` marketplace record every project on the machine shares stays on GitHub, and no `clm` is installed on `PATH`, so every other project keeps the release (`dev-install-scoped`). The shared `.claude/settings.json` is unchanged: fresh clones, teammates and adopters get the released plugin from GitHub.

Consequences:
- Every session in the clone runs the main worktree's `plugin/`, worktree sessions and builds included, because Claude Code reads the main checkout's local settings from a linked worktree. Skill edits apply at the next session or `/reload-plugins`. Keep the main checkout on `main`: on a branch that edits `plugin/`, every session runs that version.
- A worktree runs `origin/main`'s build of `clm`, not its branch's, so a branch isn't judged by the `clm` it is changing.
- Released users still run the release, so a skill change that only breaks against the released binary won't show here. Releases and the plugin's eval cases still matter.
- `clm --version` names the commit it was built from (`dev-install-script`), which tells which build is installed. For a few minutes after `origin/main` moves, the old binary still runs.
- To go back to the release, run `scripts/dev-install.sh --remove`.

Nothing that ships to adopters depends on this setup. The hook, the binary and the settings live only in the clone, written by the tracked script, which a human runs once per clone; a session never runs it.

Alternatives considered:
- `claude --plugin-dir <checkout>/plugin` with `clm@clm` turned off locally. It works per session, but needs a launch flag every time, and unattended builds wouldn't get it.
- Redefining the shared `clm` marketplace as a directory at the checkout, with the binary in `~/.cargo/bin`. It was the first setup. Every project on the machine then ran the checkout's plugin and `main`'s binary, ahead of the plugin's shim on `PATH` (`dev-install-scoped`).
- A `post-merge` hook in the main worktree to rebuild the binary. `main` moves through `clm land`'s push from the worktree that lands, so the main worktree never merges and the hook never runs; a landed change that added something the stale binary couldn't parse cost whole sessions (`rebuild-on-origin-main`).
- Releasing after each plugin change. Every release costs a human dispatch, and the binary was already tracking `main`.
