---
title: "Settings come from the environment, the repository's clm.toml, the user's config.toml, then defaults"
date: 2026-10-22
status: accepted
---

CLM's settings are read per key from, highest priority first: an environment variable, the repository's committed `clm.toml`, the user's `${XDG_CONFIG_HOME:-$HOME/.config}/clm/config.toml`, and a built-in default. The first setting, `defects`, needs both a per-user preference and a per-repository rule: a client project can commit `defects = "off"`, and no one's personal file overrides it. The variable stays on top so one session or a test can override anything, which keeps `CLM_AUTOSYNC=0` working as `autosync` moves into the settings.

The repository file is `clm.toml` at the root rather than `.clm/config.toml`, because `clm run` keeps all of `.clm/` (worktrees, logs, plans) out of git through `info/exclude`. TOML matches `Cargo.toml`. An unknown key or bad value is an error naming the file and key, so a typo is never silently ignored. Settings are committed files or the user's own, so `git-is-truth` holds.

One setting names a person rather than a preference: `writer`, the person part of the ledger writer identity (`per-writer-ledgers`). It is user-only, the mirror image of the repository-only keys: its variable or the user's `config.toml` sets it, and `clm.toml` setting it is an error. Every clone shares `clm.toml`, so a committed `writer` would make everyone write as that one person and their ledgers would collapse into one. Putting the user's file above `clm.toml` for this key alone was rejected: anyone without their own file would still take the committed name.
