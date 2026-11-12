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

**Where each command reads it.** From the tree the command runs in, with the clone's `clm.local.toml`: `clm setup` and `clm land --check-only` from the working tree, a land from its merge commit, and `clm verify` from the commit it checks out, each bisect probe from its own land. The variables are an input of the tests, like the tests' code, so they come with the tree; the rest of `[verify]` still comes from `origin/main` (`verify-settings-from-main`). Reading a land's from the root it runs in would check a branch that edits `[env]` under the main checkout's variables; reading a probe's from `main` would check every earlier land under the newest variables, so bisect could never name the land that changed one. Only the text's `[env]` is read (`committed_env`), so an old commit whose other keys a later `clm` rejects still gives its variables.

**Part of what a check proves.** A `checked` record names its command as the check preceded by `NAME=value ` for each variable, so a record written under one clone's variables never counts for a land under another's; with no `[env]` it is the check alone, as before. A land or verification whose variables differ from its base's runs every test, since a variable can change any test's result and the spec graph connects it to none (`land-checks-stay-fast`).

Rejected: a whole-table `[env]` (the drift above), and per-command tables (`[land] env` and the rest), which repeat a variable all three commands read and make changing one field copy its parent table.
