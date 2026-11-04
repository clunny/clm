---
title: "dev-install keeps the dev build and the dev plugin inside the clone"
date: 2026-11-03
status: accepted
---

A contributor wants to build CLM with its own `main` and use the release in every other project on the same machine. `dogfood-from-main` couldn't give both: it installed the binary in `~/.cargo/bin`, ahead of the plugin's shim on `PATH`, and redefined the `clm` marketplace every project shares as a directory.

**One settings file, read from every worktree.** On Claude Code 2.1.318, a linked worktree reads the main checkout's `.claude/settings.local.json`, `env` and `enabledPlugins` alike. `dogfood-from-main` rejected a separately named marketplace because worktree sessions would stay on the release; that no longer holds. So `scripts/dev-install.sh` puts the binary and a `clm-dev` marketplace in `<common>/clm-dev`, and the settings file sets `CLM_BIN`, enables `<name>@clm-dev` and turns off `<name>@clm`. The shared `clm` record stays on GitHub.

**In place, through symlinks.** A marketplace's name comes from its `marketplace.json`, so `clm-dev` needs its own. The script writes it from the tracked one and symlinks each plugin's folder to the main worktree. Claude Code loads a directory marketplace's plugins with no cached copy, so a skill edit still applies at the next session.

**The way back to GitHub changes.** `claude plugin marketplace add clunny/clm` doesn't reset a directory record. `remove` and then `add` does, and drops the installed plugin records, so setup does it only for a record that is this clone's own and says what it dropped.

**A plain `clm` finds nothing outside a session** (the human's choice). The shim is only on a session's `PATH`; a global install would be something else to keep in step, and would run ahead of the shim in every project. In the clone, `cargo run -q -p clm --` runs it.

**A worktree runs `origin/main`'s build, not its branch's.** A branch isn't judged by the `clm` it's changing, as under `dogfood-from-main`.

Consequences:
- Nothing that ships to adopters changes.
- This relies on Claude Code reading the main checkout's local settings from a linked worktree. If that stops, worktree sessions quietly run the release, with no `CLM_BIN`; `CLAUDE.md` tells a session in a set-up clone to report that.
- `--rebuild` follows the settings' `CLM_BIN`, so a clone set up earlier isn't left on a stale binary before its human runs the script again.
- Setup deletes a `clm` that `cargo install` put in `~/.cargo/bin` from a path, which earlier setups did, and leaves any other, printing that it runs ahead of the shim.
- Sessions running during setup lose their binary until restarted.
- The script now needs `claude` on `PATH`.

Alternatives considered:
- **The shim picks the binary by the current repository, and `clm run` passes `--plugin-dir`.** It doesn't rely on that Claude Code behaviour, but every adopter's shim runs an extra `git` call for a concern only contributors have, and sessions started by hand in a worktree still need an answer.
- **A `post-checkout` hook copying the settings into each new worktree.** Copies drift. It stays the fallback if Claude Code stops reading the main checkout's settings.
- **`CLAUDE_CODE_PLUGIN_DIRS` in the settings' `env`.** Tested: it loads nothing from a settings file.
