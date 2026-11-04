---
title: "This repo's sessions run main's clm and the checkout's plugin, not the release"
date: 2026-10-23
status: accepted
---

CLM is built with CLM, and its sessions should use the CLM being built, not the last release. A release lags `main` by dozens of PRs, and the plugin's skills only reach a session through a release, since Claude Code caches a GitHub-sourced plugin by `plugin.json`'s version. So on the maintainer's machine this repo runs:
- **The binary from `main`.** `.claude/settings.local.json` sets `CLM_BIN` to `~/.cargo/bin/clm`, which the plugin's `bin/clm` shim runs. An untracked `.git/hooks/post-merge` rebuilds it after a pull on `main` that changes `crates/`, `Cargo.toml` or `Cargo.lock`, in the background: `cargo install --locked --path crates/clm --target-dir target/install`, logged to `.git/clm-install.log`. (`rebuild-on-origin-main` later replaced that hook with a `reference-transaction` hook that rebuilds whenever `origin/main` moves.)
- **The plugin from the checkout.** `.claude/settings.local.json` redefines the `clm` marketplace as `{ "source": { "source": "directory", "path": "<checkout>" } }`. Claude Code then rewrites the user-level marketplace record (`~/.claude/plugins/known_marketplaces.json`) to that folder, and loads `clm@clm` from `<checkout>/plugin` in place, with no cache. The first session after the edit still runs the cached copy; later ones run the folder.

The shared `.claude/settings.json` is unchanged: fresh clones, teammates and adopters get the released plugin from GitHub.

Consequences:
- Every session on the machine runs the checkout's working tree `plugin/`, including worktree and `clm run` sessions, which don't read `settings.local.json`, because the user-level record wins. Skill edits apply at the next session or `/reload-plugins`. Keep the main checkout on `main`: on a branch that edits `plugin/`, every session runs that version.
- Released users still run the release, so a skill change that only breaks against the released binary won't show here. Releases and the plugin's eval cases still matter.
- `clm --version` doesn't change between releases, so it can't tell which build is installed. `.git/clm-install.log` shows the last rebuild, and for a minute or so after a pull the old binary still runs. After `cargo clean` the next rebuild is a full one.
- During `/clm:implement`, builds land from their own worktrees through `clm land`, so the main checkout doesn't merge and a `post-merge` hook doesn't run: the installed `clm` kept the rules of the commit it was built from for the rest of the session. This was accepted on 2026-10-30 (`20261030-clm-run-hands-out`) and then cost whole sessions, once a landed change added something the stale binary couldn't parse. `rebuild-on-origin-main` replaces the hook: the rebuild now follows `refs/remotes/origin/main`, which every land's push moves.
- To go back to the release, remove the `extraKnownMarketplaces` entry and re-add the GitHub marketplace (`claude plugin marketplace add clunny/clm`). Removing the entry alone leaves the user-level record on the folder.

Nothing checked in depends on this setup, and the hook and settings live only on the maintainer's machine. `backlog/dev-install-from-main` proposes making them reproducible.

Alternatives considered:
- `claude --plugin-dir <checkout>/plugin` with `clm@clm` turned off locally. It works per session, but needs a launch flag every time, and `clm run`'s sessions wouldn't get it.
- A separately named directory marketplace (`clm-dev`). It avoids rewriting the shared `clm` record, but keeps worktree sessions on the release, and wasn't needed once the same-name override was tested.
- Releasing after each plugin change. Every release costs a human dispatch, and the binary was already tracking `main`.
