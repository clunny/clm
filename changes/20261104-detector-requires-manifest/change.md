---
status: proposed
---

# A detector requires its manifest, and the bootstrap passes before any code exists

## Intent

`clm init --lang <lang>` on a repository with no code writes a bootstrap that fails its own checks, and a TypeScript detector whose tool isn't installed is reported as a failed detector rather than as a tool to install. This change lets a detector name the file it `requires`, which `clm check` skips it without; has `clm init` give each language's detector its manifest and guard each line of `scripts/clm-check.sh` the same way; and runs knip from `node_modules/.bin/` so a missing install reads `could not run`, with an `npm ci` hint. It comes from issues #23 and #64 (clunny/clm), accepted by the maintainer.

## Problem

Issue #23: in an empty repository, `clm init --lang typescript` writes `npx --no-install knip`, which exits 1 with nothing installed, so the bootstrap fails `check/detectors`; and for every language `scripts/clm-check.sh` runs `cargo test`, `npm ci && npm test` or `pytest`, which fail with no manifest, so the first `clm land` fails. Issue #64: in a fresh worktree of a project whose dependencies aren't installed, the same detector reports `typescript-dead-code failed (exit 1)`, which reads as a finding; `check/detectors` says `could not run …; install with:` only when the command can't start, and npx always starts.

## Approaches

See `detector-requires-manifest` for the full comparison.

- **A `requires` key on a detector, and guarded check-script lines (chosen).** Each check switches itself on when the code it needs lands; nothing has to be remembered later.
- **`clm init` writes no detector without the manifest.** Rejected: a later build has to remember to add it.
- **Read exit 127, or add the hint to every failure.** Rejected: neither catches npx's exit 1 cleanly, and the second follows every real finding with install advice.

## Outcome

- In an empty repository, `clm init --lang rust --lang typescript --lang python` leaves a result on which `clm check` and `scripts/clm-check.sh` both pass, each detector reported as skipped.
- Once `package.json` lands, the script runs `npm ci && npm test` and the knip detector runs; without `node_modules/` it reports `could not run` with `npm ci (or npm install --save-dev knip when package.json doesn't list it)`.

## Constraints

- `clm.toml` and `scripts/clm-check.sh` are the project's once written (`init-rerun`): repositories set up earlier keep their files.
- This repository's `clm.toml` doesn't use `requires`, so no `settings-before-use` ordering is needed.
