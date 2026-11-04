# Contributing to CLM

CLM is built with CLM: read `ARCHITECTURE.md`, then `CLAUDE.md`, which says how a change flows.

## Set a clone up

Run `scripts/dev-install.sh` once per clone, from any of its worktrees. It needs `jq`, `cargo` and `claude` on `PATH`, and the clone's main worktree on `main`.

It changes, and only for this clone:

- the binary and a plugin marketplace named `clm-dev`, both under the clone's git directory (`<git common dir>/clm-dev`);
- `.claude/settings.local.json` in the main worktree, which sets `CLM_BIN` and enables the `clm-dev` plugins in place of the released ones;
- the `reference-transaction` hook, which rebuilds the binary from `origin/main` in the background whenever it moves;
- Claude Code's user-level marketplace list, which gains `clm-dev`.

The clone and its worktrees then run `origin/main`'s `clm` and the main checkout's plugin, while every other project on your machine keeps running the release. Restart sessions that were already running.

## Running `clm` in the clone

A plain `clm` outside a Claude Code session finds nothing of this build. Run it from the clone with `cargo run -q -p clm --`, for example `cargo run -q -p clm -- status`.

`clm check` outside a session needs the built binary on `PATH` as well, because the `unreferenced-public-items` detector in `clm.toml` runs the first `clm` it finds there. Put `<git common dir>/clm-dev/bin` first on `PATH` for that, for example `PATH="$(git rev-parse --path-format=absolute --git-common-dir)/clm-dev/bin:$PATH" clm check`.

## Undo it

`scripts/dev-install.sh --remove` undoes the setup: the hook, `clm-dev`, the settings entries and the marketplace record. The clone's next session runs the release.

## Before a pull request

Run `cargo fmt`, `cargo clippy -- -D warnings` and `cargo test`.
