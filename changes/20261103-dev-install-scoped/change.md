---
status: applied
resolves: [dev-install-scoped-to-the-clone]
human_burdens:
  - "Once the change is built, run `scripts/dev-install.sh` once in each set-up clone, with no sessions running on it: it deletes the dev build from `~/.cargo/bin` and takes the `clm` marketplace back to GitHub, which drops Claude Code's installed `clm` plugin records, so other projects install the plugin again at their next session."
  - "A plain `clm` typed in a terminal outside a session then finds nothing, in the clone or anywhere else; in the clone, use `cargo run -q -p clm --`."
---

## Intent

A contributor's clone of CLM's repository, with all its worktrees, runs `origin/main`'s `clm` and the main checkout's plugin, and every other project on the machine runs the release, binary and plugin. Today `scripts/dev-install.sh` reaches further than its clone: the binary sits in `~/.cargo/bin`, ahead of the plugin's shim on `PATH`, and the `clm` marketplace record every project shares points at the checkout. This change keeps both inside the clone, and writes down how a contributor sets up and undoes it.

## Problem

`dogfood-from-main` redefined the shared `clm` marketplace as a directory because a separately named one would leave worktree sessions on the release. On this machine:

- `~/.cargo/bin` comes before the plugin's `bin/` on a session's `PATH`, so a plain `clm` in any project finds the dev build.
- `known_marketplaces.json` holds `clm` as a directory source at the checkout, so any project enabling `clm@clm` loads the checkout's skills.

## What we measured

On Claude Code 2.1.318, with a throwaway `CLAUDE_CONFIG_DIR`, reading each session's `init` event and a logging `SessionStart` hook:

- A linked worktree reads the main checkout's `.claude/settings.local.json`, both `env` and `enabledPlugins`, whether it lies under `.claude/worktrees/` or outside the repository's folder. `dogfood-from-main`'s reason for rejecting a separate marketplace no longer holds.
- A marketplace named `clm-dev` in the git directory, built from the tracked `marketplace.json` with its plugin folders symlinked to the checkout, loads both `clm` and `clm-panel` in place, with no cached copy. Installed at local scope, with `clm@clm` and `clm-panel@clm` set false, the main checkout and its worktrees load only the dev plugins, and another project on the same config loads the released `clm@clm` from the cache.
- `claude plugin marketplace add clunny/clm` does not reset a directory record, though `dev-install --remove` and `dogfood-from-main` both say it does. `claude plugin marketplace remove clm` and then the `add` does, and drops the installed plugin records, so each plugin has to be installed again.
- `marketplace add` and `plugin install` run again on what is already there exit 0. `plugin uninstall` of what isn't installed exits 1, as does `marketplace remove` of an unknown marketplace.
- `CLAUDE_CODE_PLUGIN_DIRS` set in a settings file's `env` loads nothing, so it can't scope the plugin.

These ran without signing in, so no model turn ran: a `clm` call through the shim of the `clm-dev` plugin wasn't exercised. The first real session after setup confirms that.

## Approaches

- **Everything in the clone's settings and git directory (chosen).** The binary under `<common>/clm-dev`, `CLM_BIN` pointing at it, a `clm-dev` marketplace enabled in `settings.local.json`. One file covers the clone and its worktrees, and nothing that ships to adopters changes. It relies on Claude Code reading the main checkout's local settings from a linked worktree.
- **The shim picks the binary by the current repository, and `clm run` passes `--plugin-dir`** (the item's first idea). It works whatever Claude Code does with local settings, but every adopter's shim runs an extra `git` call for a concern only CLM's contributors have, and sessions started by hand in a worktree still need an answer. Rejected: product code for what a setting already does.
- **A `post-checkout` hook copying the settings into each new worktree.** It covers a Claude Code that stops reading the main checkout's settings, but copies drift and need refreshing. Rejected until that happens.
- **Only document it.** It leaves the leak. Rejected.

## Outcome

- `scripts/dev-install.sh` installs into `<common>/clm-dev/bin`, builds a `clm-dev` marketplace at `<common>/clm-dev/marketplace`, installs its plugins at local scope, turns the `@clm` ones off in `settings.local.json`, and no longer writes `extraKnownMarketplaces.clm`.
- It takes a `clm` record that is a directory source at this clone back to GitHub, and uninstalls a `clm` that `cargo install` put in place from a path.
- `--rebuild` installs where the settings' `CLM_BIN` points, so a clone set up before this keeps working until its human runs the script again.
- `--remove` undoes all of it, and deletes `<common>/clm-dev`.
- `CONTRIBUTING.md` says how to set up, what changes on the machine, how to undo it, and what a plain `clm` does.

## Constraints

- A plain `clm` in a terminal is left out: it finds nothing (the human's choice), and nothing here adds a wrapper or a global install.
- A worktree on a branch runs `origin/main`'s binary and the main checkout's plugin, not its own branch's code: a branch isn't judged by the `clm` it's changing.
- No session runs the script (`no-shared-install`); only `CLAUDE.md`'s instruction to tell the human changes.
- The plugin (`plugin/`, the shim) doesn't change.
