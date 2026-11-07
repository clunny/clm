---
component: upgrade
---

## ADDED Requirements

### Requirement: migration-unattended-allow-rules
An `advised` note migration, ID `20261107-projects-set-up-before`, SHALL offer a project the allow rules for unattended `clm` commands that `clm init` seeds on a first run (`init/init-seeds-permissions`) and that the project lacks. It SHALL hold its own copy of those rules as that requirement lists them when this migration ships, since `upgrade` doesn't depend on `init`: for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, twenty-two in all. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds every one of those rules, read and compared as `init/init-check-permissions` reads and compares them: as exact strings, a file that isn't JSON of that shape allowing nothing; both paths are plain paths under the root, not `config`'s `local_settings_path`, as `clm init --check-permissions` reads them. It parses the JSON with the workspace's `serde_json`, which `clm-upgrade` then depends on. Its `file` SHALL be `.claude/settings.json`, and it SHALL carry no `old` or `new`, so `clm upgrade --apply` never writes the settings: they are the human's, and a rule a project removed on purpose is not put back (`unattended-steps-run-clm`). Its note's first line SHALL read `Unattended sessions need clm's allow rules in .claude/settings.json`. The note SHALL list the rules; say that `clm init --check-permissions` prints those the project lacks and that the human adds them to `.claude/settings.json` by hand; say that `clm upgrade --decline 20261107-projects-set-up-before` keeps a project that leaves rules out on purpose as it is; and say that when the root component's `paths` don't list `.claude/settings.json`, as in a project set up before `clm init` seeded the rules, a new settings file needs adding there through a spec change before `clm check` passes. The capability test `capability/upgrade-clm`'s *skipping releases* takes this note's edit as `/clm:upgrade` would have the human take it: it writes the rules to `.claude/settings.json` and adds the file to the root's `paths`, then expects `up to date`. So do the other tests whose fixture stands for an up-to-date project and that this migration would leave pending: `crates/clm-upgrade/src/shipped_tests.rs`'s `project()` and the `clm_repo` fixture of `crates/clm/tests/hooks.rs`'s `session_start_reminds_to_upgrade` give their project a `.claude/settings.json` allowing the rules.

#### Scenario: a project set up before the rules
- GIVEN a project with no `.claude/settings.json` and no `.claude/settings.local.json`
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261107-projects-set-up-before advised note Unattended sessions need clm's allow rules in .claude/settings.json`, the second prints `note 20261107-projects-set-up-before`, and `.claude/` still doesn't exist

#### Scenario: a project lacking only the newest rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every rule but `Bash(clm verify)` and `Bash(clm verify *)`
- WHEN `clm upgrade --apply` runs
- THEN it prints `note 20261107-projects-set-up-before`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing every rule but `Bash(clm land)`, and a `.claude/settings.local.json` allowing `Bash(clm land)`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Scenario: the older rule form
- GIVEN a `.claude/settings.json` allowing every rule but the two `clm land` ones, and `Bash(clm land:*)`
- WHEN `clm upgrade` runs
- THEN it lists the migration

#### Scenario: declined
- GIVEN a project lacking the rules, whose `clm.toml` sets `declined_migrations = ["20261107-projects-set-up-before"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command
- init/init-seeds-permissions
- init/init-check-permissions
