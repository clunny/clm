---
component: clm
---

## MODIFIED Requirements

### Requirement: upgrade-corpus <!-- base: dfc50a6d9aa115f8 -->
For every entry under `crates/clm-upgrade/corpus/`, a copy upgraded with the built `clm upgrade --apply`, and then, when the `clm upgrade` listing run after that names `20261107-projects-set-up-before` as pending, with `clm upgrade --decline 20261107-projects-set-up-before` (`upgrade/migration-unattended-allow-rules`, whose file the comparison leaves out), SHALL be a project where `clm upgrade` prints `up to date`, `clm check` passes, and each file `clm init --lang rust --no-detectors --host none` writes, but `.claude/settings.json` and `specs/SPEC.md`, equals what the built `clm init` writes with those flags in a directory named `project`, as each entry's is. An entry written by a `clm init` that already seeds the allow rules (`init/init-seeds-permissions`) doesn't have that migration pending, and `clm upgrade --decline` refuses a migration that isn't pending, so the test declines it only for an entry that has it pending; a release records its own entry before its tests run (`ci/release-records-upgrade-corpus`), so every entry from then on is such an entry. `clm.toml` SHALL be compared as parsed TOML, its `upgraded` and `declined_migrations` keys aside. The two files are left out (`corpus-skips-first-run-files`) because, unlike the others, `clm init` writes them only on a first run, before the repository has a `specs/SPEC.md`, and their text depends on the repository it ran in (the root's title and `outside`, the settings Claude Code already held), so no template text exists for a migration's `old` and `new`: a project set up before `init/init-seeds-permissions` has no settings file, so its root component rightly lists none in its `paths`, and the allow rules it lacks are offered by a note migration, which the test declines, since taking it is a hand edit, and checked by `upgrade-covers-allow-rules` instead. The test SHALL take the expected files from a function it is given, so a test can show that a template changed without a migration fails it. It lives in `crates/clm/tests/upgrade.rs`, since it runs the built binary.

#### Scenario: every release upgrades
- GIVEN the corpus entries `v0.14.0` to the latest release
- WHEN each is upgraded so
- THEN each ends `up to date`, passes `clm check`, and holds the files `clm init` writes today, `.claude/settings.json` and `specs/SPEC.md` aside

#### Scenario: an entry set up with the allow rules
- GIVEN an entry packed from a project the built `clm init --lang rust --no-detectors --host none` wrote in a directory named `project`, so `20261107-projects-set-up-before` isn't pending for it
- WHEN the test's check runs on it
- THEN it runs no `clm upgrade --decline`, and the entry passes: `up to date`, `clm check` passing and the files equal

#### Scenario: a template changed without a migration
- GIVEN the corpus, and an expected `CLAUDE.md` with one word changed
- WHEN the test's check runs against it
- THEN it fails, naming `CLAUDE.md` and the corpus entry

#### Needs
- upgrade/upgrade-apply
- upgrade/upgrade-note-and-decline
- upgrade/corpus-seeded
- upgrade/migration-carried-constraints-copy
- upgrade/migration-claude-md-names-help
- upgrade/migration-check-script-from-root
- upgrade/migration-unattended-allow-rules
- config/upgrade-settings
- init/init-stamps-upgraded
