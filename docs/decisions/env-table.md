---
title: "[env] sets variables for the commands CLM runs, merged name by name, over the caller's environment"
date: 2026-11-12
status: accepted
---

`[env]` in `clm.toml` or `clm.local.toml` names variables that CLM sets in the environment of `[build] setup`, `[land] check` and `[verify] check` (`20261112-config-env`).

**Merged name by name.** Every other table comes whole from one layer (`local-settings`), so that what a file says is all of that table. `[env]` is a set of independent variables, not one setting with parts that depend on one another: a clone that needs a different container image for its architecture must be able to change that variable alone, without copying the rest and drifting from `clm.toml` when they change. That drift, copying a whole `[verify]` to change what one variable gives, is the problem the table exists to solve. Each variable keeps the layer it came from, so errors and `clm config list` still name one file per value.

**Over the caller's environment.** A variable `[env]` sets replaces the one the shell or session that started `clm` holds. The checks should see what the project's files say, not what a session happened to export; a shell profile or Claude Code's `settings.local.json` `env` would otherwise decide a land differently from a terminal run. A one-off value goes in `clm.local.toml`, which takes effect at once.

**Repository-only.** Only `clm.toml` and `clm.local.toml` may set it, as for `[land]` and `[verify]`, so the user's `config.toml` can't change the checks of every project it reaches. `clm.local.toml` changing a variable a check reads gives a clone no power it lacks: it can already replace `[land] check` outright, and it belongs to the clone that runs the check.

**CLM's own names refused.** Names beginning `CLM_`, and `CI`, are ones CLM sets or removes for its runs (`CLM_SLOT`, `CLM_SELECTION`, `CLM_LAND_DIR`, a land's removal of `CI`). Loading refuses them rather than letting CLM's own value silently win.

**Where each command reads it.** From the same settings as the command it serves: a land and `clm setup` from the root they run in, `clm verify` from `origin/main`'s `clm.toml` with the clone's `clm.local.toml` (`verify-settings-from-main`), so the shared variables of a verify run come from `main` and this machine's from the clone.

Rejected: a whole-table `[env]` (the drift above), and per-command tables (`[land] env` and the rest), which repeat a variable all three commands read and make changing one field copy its parent table.
