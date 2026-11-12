---
status: proposed
---

## Intent

Let `clm.toml` and `clm.local.toml` set environment variables for the commands CLM runs. A new `[env]` table holds variables that CLM sets in the environment of `[build] setup`, `[land] check` and `[verify] check`. `clm.local.toml`'s variables replace `clm.toml`'s name by name, the one table whose layers merge, which the settings guide says. A clone can then change one variable a check reads without copying a whole table or touching every project's shell.

## Problem

A project's slow-test script reads `RELAY_SERVER_IMAGE`, a locally built container image, with a default that suits one machine's architecture. On a different machine the only overrides today are:
- a shell profile, which leaks into every project and every shell;
- Claude Code's `.claude/settings.local.json` `env`, which reaches only commands run inside Claude sessions, not a land or verify run from a terminal;
- redefining the whole `[verify]` table in `clm.local.toml`, since tables never merge (`local-settings`), which copies `mode`, `triggers` and `every` and drifts from `clm.toml` when they change.

## Approaches

1. **An `[env]` table, merged name by name, that wins over the caller's environment** (chosen). The project states the variables its commands need in `clm.toml`; one clone replaces only what differs. The three commands already run through two places that set variables (`land`'s shared check runner, which `verify` uses, and `clm setup`), so the variables go in beside `CLM_SLOT` and `CLM_LAND_DIR`.
2. **Whole-table `[env]`, as every other table.** Simpler to state, but a clone overriding one variable must copy the rest, the same drift as copying `[verify]`.
3. **Per-command tables (`[land] env`, `[verify] env`, `[build] env`).** Finer control, but a variable the setup, the land check and the slow tests all read is written three times, and changing one field would copy the whole parent table, which is the problem itself. If a project ever needs a variable for one command only, a later change can add a per-command table on top of this one.

## Outcome

- `config/env-setting` (added): `env` is repository-only, set only by `clm.toml` or `clm.local.toml`, so neither the user's `config.toml` nor a shell variable can change a project's checks from outside its clone. It is a table of strings keyed by variable names. Loading refuses names beginning `CLM_` and `CI`, which CLM sets or removes for its runs, so a variable in the table can never be silently overridden. It also refuses values holding a line break, since `clm config get` prints one line per variable. The two files' tables merge name by name, each variable keeping the file it came from.
- `config/layers` and `config/config-list` (modified): `env` is the exception to tables never merging, and `clm config list` prints one `env` line per variable with its file. `config`'s Contract and Design state both; the guide's settings page gains `env`'s section and says the exception where it describes tables.
- `land/check-env` (added): every run of `[land] check`, by a land or `--check-only`, sets the variables, from the settings it takes `[land] check` from (the root's, `land-policy-from-base`).
- `verify/verify-env` (added): every run of `[verify] check`, bisect probes included, sets the variables from the settings it follows: `origin/main`'s `clm.toml` with the clone's `clm.local.toml` (`settings-from-main`), never the working tree's `clm.toml`.
- `orchestrator.run/setup-command` (modified): `clm setup` sets the variables.

In all three commands, a variable in `[env]` replaces the value the caller's environment holds (`env-table`), and CLM's own variables are applied after it, unchanged.

## Constraints

- `deterministic-enforcement`: the reserved names and the repository-only rule are checked by loading, not by prose.
- `settings-before-use`: this repository's `clm.toml` sets no `[env]`, so no requirement here sets the value and no install item is needed.
- Out of scope: `clm check`'s detectors and other commands CLM runs (such as `[[detectors]]`) get no `[env]`. The table can't unset a variable the caller's environment holds.
