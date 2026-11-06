---
status: applied
resolves: [corpus-entries-drift-from-init-without-a-migration]
---

# Corpus comparison: a migration for clm-check.sh, first-run files left out

## Intent

Make `clm/upgrade-corpus` buildable. A build of its test found three files where every corpus entry, upgraded, still differs from what `clm init` writes today, with no migration for any of them: `.claude/settings.json`, the generated root `specs/SPEC.md` (whose `paths` now list that settings file) and, for `v0.14.0` and `v0.15.0`, `scripts/clm-check.sh`. As the human chose, this change splits them: `upgrade` gains an advised note migration for `scripts/clm-check.sh`, which rewrites the script only while it still equals the old template text; and the corpus comparison leaves out `.claude/settings.json` and `specs/SPEC.md`, which `clm/upgrade-corpus` now names as written on a first run only, with settings drift left to `clm init --check-permissions`.

## The human's answers

> Split (Recommended): Add an advised note migration for clm-check.sh (rewritten only when it still equals the old template text). Leave settings.json and the root SPEC.md out of the comparison, naming them in the requirement as written on first run only; settings drift stays reported by `clm init --check-permissions`.

## Problem

`clm/upgrade-corpus` requires each upgraded entry to equal, file by file, what `clm init --lang rust --no-detectors --host none` writes today. Three files fail that with no migration, so the requirement as written can't pass and was left unbuilt (`backlog/corpus-entries-drift-from-init-without-a-migration`).

- `scripts/clm-check.sh` is a real drift in a file older projects hold: `v0.14.0` and `v0.15.0` wrote a bare `cargo test`; today's script first `cd`s to the repository's root and runs `cargo test` only when `Cargo.toml` exists. `upgrades-carry-migrations` wants a migration for it.
- `.claude/settings.json` and the root `specs/SPEC.md` are written on a first run only (`init-seeds-permissions`, `init-root-ownership`), and belong to the project once written (`land-locally`). A migration would edit files the project has since changed, and a project set up before `init-seeds-permissions` holds no settings file, so its root rightly lists none in `paths`.

## Approaches

- **Split (chosen by the human).** A note migration where the drift is a template the project still holds unedited; the first-run files left out of the comparison, with the reason written into the requirement.
- **A migration for each of the three.** An apply migration adding `.claude/settings.json` to the root's `paths` and seeding the settings file would rewrite files the project owns and may have changed, against `land-locally`; dropped.
- **Compare only the files a migration owns.** Would leave `ARCHITECTURE.md`'s and `CLAUDE.md`'s untouched siblings (`CAPABILITIES.md`, `.gitignore`, `clm.toml`) unguarded, so a template change to them could ship with no migration; dropped.

## Outcome

- `upgrade/migration-check-script-from-root`: an advised note migration, ID `20261106-corpus-entries-drift-from`, whose `done` is true when `scripts/clm-check.sh` is absent or holds the `cd "$(dirname "${BASH_SOURCE[0]}")/.."` line. Its `old` is the `v0.14.0`/`v0.15.0` script for `--lang rust` (the corpus's language), its `new` today's; any other script (another language, or one the project edited) gets the note, which says what changed for every language.
- `clm/upgrade-corpus`, modified: the comparison leaves out `.claude/settings.json` and `specs/SPEC.md`, saying why; states that the expected files come from `clm init` in a directory named `project`, as the entries' is, and that `clm.toml` is compared as parsed TOML, its `upgraded` key aside (`clm upgrade --apply` places `upgraded` where `clm init` doesn't); its first scenario now checks the comparison too; its `Needs` adds the new migration.

## Constraints

- `upgrades-carry-migrations` holds: the one drift in a file an older project holds and CLM may rewrite gets a migration. The settings file is not held by older projects at all, and the root spec's `paths` list it only because it exists in a fresh project.
- `shipped-migrations-frozen`: the new migration is shipped by the next release and locked then; no shipped migration changes.
- No `init` change: `clm init` keeps writing what it writes.

Known edge cases: a project whose script was set up for a language other than Rust, or for several, is never rewritten automatically, since one note carries one `new` text; its note says what to change. No workarounds are known.

## Open questions

None.
