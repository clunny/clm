---
component: clm
---

## MODIFIED Requirements

### Requirement: upgrade-corpus <!-- base: d9ae7ae2c51cbed7 -->
For every entry under `crates/clm-upgrade/corpus/`, a copy upgraded with the built `clm upgrade --apply`, and then with `clm upgrade --decline 20261107-projects-set-up-before` (`upgrade/migration-unattended-allow-rules`, whose file the comparison leaves out), SHALL be a project where `clm upgrade` prints `up to date`, `clm check` passes, and each file `clm init --lang rust --no-detectors --host none` writes, but `.claude/settings.json` and `specs/SPEC.md`, equals what the built `clm init` writes with those flags in a directory named `project`, as each entry's is. `clm.toml` SHALL be compared as parsed TOML, its `upgraded` and `declined_migrations` keys aside. The two files are left out (`corpus-skips-first-run-files`) because, unlike the others, `clm init` writes them only on a first run, before the repository has a `specs/SPEC.md`, and their text depends on the repository it ran in (the root's title and `outside`, the settings Claude Code already held), so no template text exists for a migration's `old` and `new`: a project set up before `init/init-seeds-permissions` has no settings file, so its root component rightly lists none in its `paths`, and the allow rules it lacks are offered by a note migration, which the test declines, since taking it is a hand edit, and checked by `upgrade-covers-allow-rules` instead. The test SHALL take the expected files from a function it is given, so a test can show that a template changed without a migration fails it. It lives in `crates/clm/tests/upgrade.rs`, since it runs the built binary.

#### Scenario: every release upgrades
- GIVEN the corpus entries `v0.14.0` to the latest release
- WHEN each is upgraded so
- THEN each ends `up to date`, passes `clm check`, and holds the files `clm init` writes today, `.claude/settings.json` and `specs/SPEC.md` aside

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

#### Serves
- architecture/upgrades-carry-migrations
- capability/upgrade-clm

## ADDED Requirements

### Requirement: upgrade-covers-allow-rules
For each rule of `clm-init`'s `UNATTENDED_RULES`, the list `init/init-seeds-permissions` seeds, a repository the built `clm init --lang rust --no-detectors --host none` has just set up, with that one rule removed from `.claude/settings.json`'s `permissions.allow`, SHALL be one where the built `clm upgrade` lists a pending migration. So a change that adds a rule to `clm init`'s seeding fails a land until it ships a migration that offers the rule to projects set up earlier (`upgrades-carry-migrations`), which the corpus test, leaving the settings file out (`corpus-skips-first-run-files`), can't see. The test SHALL take the rule list from an argument, so a test can show that a rule no migration offers fails it. It lives in `crates/clm/tests/upgrade.rs`, since it runs the built binary.

#### Scenario: every seeded rule is offered
- GIVEN `UNATTENDED_RULES`
- WHEN the test's check runs for each of them
- THEN for every one, `clm upgrade` on the repository lacking it lists a pending migration

#### Scenario: a rule no migration offers
- GIVEN a repository the built `clm init` has just set up, with `Bash(clm example)` added to its `.claude/settings.json`, and the rule list `Bash(clm example)`
- WHEN the test's check runs on that list
- THEN it fails, naming `Bash(clm example)`

#### Needs
- upgrade/migration-unattended-allow-rules
- init/init-seeds-permissions

#### Serves
- architecture/upgrades-carry-migrations
