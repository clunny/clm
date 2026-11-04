---
title: "A detector requires its language's manifest, and init's check script waits for it too"
date: 2026-11-04
status: accepted
---

`clm init --lang <lang>` on a repository with no code wrote a detector and a `scripts/clm-check.sh` line that assume the language's project exists. For TypeScript, `npx --no-install knip` exits 1 with nothing installed, so the bootstrap failed `check/detectors`; for every language the check script's `cargo test`, `npm ci && npm test` or `pytest` fails with no manifest, so the first land failed (issue #23). And in a fresh worktree of a project whose `package.json` lists knip, the same npx call reported `typescript-dead-code failed (exit 1)`, a finding, when the tool simply wasn't installed (issue #64): `check` reports `could not run …; install with: <hint>` only for a command that can't start, and npx always starts.

Now a `[[detectors]]` table may set `requires`, a path relative to the repository root. The `clm check` command skips the detector, saying so on stderr, while nothing exists there; the root does the skip, since it builds `clm-check`'s detector list, so `clm-check`'s `Detector` doesn't change and no build reaches into another component (`path-ownership`, which allows such fixes only for a removal). `clm init` gives each language's detector its manifest (`Cargo.toml`, `package.json`, `pyproject.toml`), and its check script guards each language's line the same way, so the bootstrap of an empty repository passes, and each check switches itself on when the code that needs it lands, with no follow-up step to remember. The TypeScript detector runs `node_modules/.bin/knip` directly, found from the repository root, so a missing install is `could not run`, with a hint that says `npm ci`.

A Python project with only a `setup.py` skips its detector until it gains a `pyproject.toml`, or the project edits `requires`; `clm.toml` is the project's file once written.

Rejected:
- **`clm init` writes no detector for a language whose manifest is missing,** as with `--no-detectors`, and the first build adds it. No new key, but the detector then waits on a later build remembering to add it, and the manifest is often written by a component's build, not the root's.
- **Treat exit 127 as `could not run`.** It would catch a tool missing behind `sh -c`, but not npx's exit 1, and it would misread a tool that exits 127 itself.
- **Append the hint to every `failed` line.** It would follow every real finding with install advice.
