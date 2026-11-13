---
component: upgrade
---

## ADDED Requirements
### Requirement: migration-build-allow-rules
An `advised` note migration, ID `20261112-build-loop-in-clm`, SHALL offer a project the two allow rules for `clm build` that `init/init-seeds-permissions` adds to `clm init`'s seeding, `Bash(clm build)` and `Bash(clm build *)`, when the project lacks either. It SHALL hold its own copy of the two rules, since `upgrade` doesn't depend on `init`, and the shipped migrations that offer the other rules stay as they are (`shipped-migrations-frozen`). Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds both rules, read and compared as `migration-unattended-allow-rules` reads and compares them. Its `file` SHALL be `.claude/settings.json`, and it SHALL carry no `old` or `new`, so `clm upgrade --apply` never writes the settings (`unattended-steps-run-clm`, `allow-rules-offered-by-note`). Its note's first line SHALL read `Unattended sessions need clm's allow rules for clm build in .claude/settings.json`. The note SHALL list the two rules; say that `/clm:build` runs every step of its loop through `clm build` and that Claude Code's auto mode denies a step when no rule allows it; say that the human adds them to `.claude/settings.json` by hand and that `clm init --check-permissions` prints the rules the project lacks; and say that `clm upgrade --decline 20261112-build-loop-in-clm` keeps a project that leaves them out on purpose as it is. `init/init-seeds-permissions`' build writes this migration (`template-change-ships-its-migration`), and gives the fixtures that stand for an up-to-date project the two rules, as `migration-setup-allow-rules` states for its own: `crates/clm-upgrade/src/shipped_tests.rs`'s `project()`, the `clm_repo` fixture of `crates/clm/tests/hooks.rs`'s `session_start_reminds_to_upgrade`, and the capability test `capability/upgrade-clm`'s *skipping releases*; and it adds this migration's ID to the allow-rules migrations `crates/clm/tests/upgrade.rs`'s corpus test declines when pending (`clm/upgrade-corpus`), naming those edits under `## Fixes in other components` in its pull request.

#### Scenario: a project lacking the build rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every other rule `clm init` seeds but neither `clm build` rule
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261112-build-loop-in-clm advised note Unattended sessions need clm's allow rules for clm build in .claude/settings.json`, the second prints `note 20261112-build-loop-in-clm`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing `Bash(clm build)`, and a `.claude/settings.local.json` allowing `Bash(clm build *)`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-build-loop-in-clm`

#### Scenario: declined
- GIVEN a project lacking the build rules, whose `clm.toml` sets `declined_migrations = ["20261112-build-loop-in-clm"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-build-loop-in-clm`

#### Needs
- init/init-seeds-permissions
