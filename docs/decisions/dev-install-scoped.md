---
title: "dev-install keeps the dev build and the dev plugin inside the clone"
date: 2026-11-03
status: accepted
---

A contributor wants to build CLM with its own `main` and use the release in every other project on the same machine. The first setup of `dogfood-from-main` couldn't give both: it installed the binary in `~/.cargo/bin`, ahead of the plugin's shim on `PATH`, and redefined the `clm` marketplace every project shares as a directory.

**One settings file, read from every worktree.** On Claude Code 2.1.318, a linked worktree reads the main checkout's `.claude/settings.local.json`, `env` and `enabledPlugins` alike, so a separately named marketplace no longer leaves worktree sessions on the release. So `scripts/dev-install.sh` puts the binary in `<common>/clm-dev` and adds a `clm-dev` marketplace, and the settings file sets `CLM_BIN`, enables `<name>@clm-dev` and turns off `<name>@clm`. The shared `clm` record stays on GitHub.

**The marketplace is the main worktree.** A marketplace's name comes from its manifest, so `clm-dev` needs its own. The script writes the tracked manifest, renamed `clm-dev`, to `<main worktree>/.clm/dev-marketplace.json` and adds that file, so the plugins lie inside the marketplace and are read from the main worktree in place (`dev-marketplace-in-place`). Claude Code loads them with no cached copy, so a skill edit still applies at the next session.

**The way back to GitHub changes.** `claude plugin marketplace add clunny/clm` doesn't reset a directory record. `remove` and then `add` does, and drops the installed plugin records, so setup does it only for a record that is this clone's own and says what it dropped.

**A plain `clm` finds nothing outside a session** (the human's choice). The shim is only on a session's `PATH`; a global install would be something else to keep in step, and would run ahead of the shim in every project. In the clone, `cargo run -q -p clm --` runs it.

**A worktree runs `origin/main`'s build, not its branch's.** A branch isn't judged by the `clm` it's changing (`dogfood-from-main`).

Consequences:
- Nothing that ships to adopters changes.
- This relies on Claude Code reading the main checkout's local settings from a linked worktree. If that stops, worktree sessions quietly run the release, with no `CLM_BIN`; `CLAUDE.md` tells a session in a set-up clone to report that.
- `--rebuild` follows the settings' `CLM_BIN`, so a clone set up earlier isn't left on a stale binary before its human runs the script again.
- Setup deletes a `clm` that `cargo install` put in `~/.cargo/bin` from a path, which earlier setups did, and leaves any other, printing that it runs ahead of the shim.
- Sessions running during setup lose their binary until restarted.
- The script now needs `claude` on `PATH`.
- Outside a session no `clm` is on `PATH`, so `clm check` there, through `cargo run`, needs `<common>/clm-dev/bin` on `PATH` for its `unreferenced-public-items` detector. `CONTRIBUTING.md` says so.

Alternatives considered:
- **The shim picks the binary by the current repository, and `clm run` passes `--plugin-dir`.** It doesn't rely on that Claude Code behaviour, but every adopter's shim runs an extra `git` call for a concern only contributors have, and sessions started by hand in a worktree still need an answer.
- **A `post-checkout` hook copying the settings into each new worktree.** Copies drift. It stays the fallback if Claude Code stops reading the main checkout's settings.
- **`CLAUDE_CODE_PLUGIN_DIRS` in the settings' `env`.** Tested: it loads nothing from a settings file.
