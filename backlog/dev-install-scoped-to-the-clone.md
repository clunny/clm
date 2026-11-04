---
title: "dev-install affects only its own clone: the binary inside the clone, a clm-dev marketplace, and contributor docs"
kind: idea
components: [clm, plugin]
---

## Intent

A contributor's clone of CLM's repository runs its own build of `clm` and the checkout's plugin. Every other project on that machine that builds with CLM runs the release, both binary and plugin. Today `scripts/dev-install.sh` reaches further than its clone.

## Where it leaks

- **The binary.** It installs to `~/.cargo/bin/clm`, which can come before the plugin's shim on `PATH`, so a plain `clm` in another project can find the dev build. `CLM_BIN` itself is set only in the clone's `.claude/settings.local.json`, so calls through the shim there get the release.
- **The plugin.** Redefining the `clm` marketplace as a directory makes Claude Code rewrite the user-level record, so every session on the machine loads the checkout's `plugin/` (`dogfood-from-main`).

This comes from `dogfood-from-main` and the `PATH` order seen in one session. No other project on the maintainer's machine uses CLM, so confirm it in a real one first.

## Ideas so far

- **Binary inside the clone.** Install it under the clone, for example with `cargo install --root` beneath the git common directory, and have the shim run it only when the current repository is that clone. Worktree sessions don't read `settings.local.json`, and probably reach the dev build today through the `PATH` entry this removes, so the shim's check has to cover them.
- **A `clm-dev` marketplace.** A separately named directory marketplace leaves the shared `clm` record on the release. `dogfood-from-main` rejected this because worktree sessions would stay on the release, so `clm run` would need to load the checkout's plugin itself, for example with `--plugin-dir`, and interactive worktree sessions need an answer too.
- **Docs.** A short `CONTRIBUTING.md` saying to run `scripts/dev-install.sh` once per clone, what it changes on the machine, and how `--remove` undoes it. Today only `CLAUDE.md` mentions it.

This reverses part of `dogfood-from-main`, so it needs a spec change. `--remove` and the rebuild hook (`rebuild-on-origin-main`) need to follow the new locations.
