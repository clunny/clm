---
component: clm
---

## MODIFIED Requirements

### Requirement: upgrade-corpus <!-- base: c3673f3091533558 -->
For every entry under `crates/clm-upgrade/corpus/`, a copy upgraded with the built `clm upgrade --apply` SHALL be a project where `clm upgrade` prints `up to date`, `clm check` passes, and each file `clm init --lang rust --no-detectors --host none` writes, but `.claude/settings.json` and `specs/SPEC.md`, equals what the built `clm init` writes with those flags in a directory named `project`, as each entry's is. `clm.toml` SHALL be compared as parsed TOML, its `upgraded` key aside. The two files are left out because `clm init` writes them on a first run only, the project owns them from then on, and no migration rewrites them: a project set up before `init/init-seeds-permissions` has no settings file, so its root component rightly lists none in its `paths`, and `clm init --check-permissions` reports the allow rules it lacks. The test SHALL take the expected files from a function it is given, so a test can show that a template changed without a migration fails it. It lives in `crates/clm/tests/upgrade.rs`, since it runs the built binary.

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
- upgrade/corpus-seeded
- upgrade/migration-carried-constraints-copy
- upgrade/migration-claude-md-names-help
- upgrade/migration-check-script-from-root
- config/upgrade-settings
- init/init-stamps-upgraded
