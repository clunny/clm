---
title: "Corpus entries differ from what clm init writes today in files no migration covers"
kind: question
components: [clm, upgrade, init]
blocks: [clm/upgrade-corpus]
---

`clm/upgrade-corpus` says a corpus entry upgraded with `clm upgrade --apply` must equal, file by file, what the built `clm init --lang rust --no-detectors --host none` writes (`clm.toml`'s `upgraded` line aside). Its `Needs` name only the two migrations for `ARCHITECTURE.md` and `CLAUDE.md`. A build of the test, run against the five entries `v0.14.0` to `v0.18.0` with the project name normalised (corpus entries are named `project`, a fresh init takes its directory's name) and `clm.toml` compared as parsed TOML, shows that three more files differ, each with no migration, so the requirement as written cannot pass and was left unbuilt:

- `.claude/settings.json`: absent from all five entries; `clm init` writes it today (the allow rules for unattended `clm` commands, `init-seeds-permissions`), and it is on the root component's `paths` in `specs/SPEC.md`.
- `specs/SPEC.md`: the root's `paths` gains `.claude/settings.json` (all five entries lack it).
- `scripts/clm-check.sh`: `v0.14.0` and `v0.15.0` hold a bare `cargo test`; today's template starts with `cd "$(dirname "${BASH_SOURCE[0]}")/.."` and runs `cargo test` only `if [ -f Cargo.toml ]`.

The corpus test is doing its job here: `upgrades-carry-migrations` says a change to what `clm init` writes ships the migration, and these three changes shipped none.

What a spec change must decide:

1. Which of the three gets a migration (a `required` or `advised` note migration in `upgrade`, with `old` and `new` texts, or an apply migration for the settings file and the `paths` line), so that `upgrade` holds one per drift and `clm/upgrade-corpus` names each in its `Needs`.
2. Or whether the requirement's comparison covers only the files a migration owns, in which case its text and scenarios say which files are left out and why (for example, a settings file seeded on a first run only).

The test is otherwise ready to build: `crates/clm/tests/upgrade.rs` gains `check_entry(tarball, expected)`, which unpacks an entry, commits it, runs `clm upgrade --apply`, requires `clm upgrade` to print `up to date` and `clm check` to pass, and compares the files from `expected()`, a function it is given, so a second test can pass it a `CLAUDE.md` with a word added and see it fail naming the file and entry.
