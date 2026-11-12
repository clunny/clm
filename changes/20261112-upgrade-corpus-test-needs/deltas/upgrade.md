---
component: upgrade
---

## MODIFIED Requirements
### Requirement: migration-setup-allow-rules <!-- base: dfae9b68663d12f5 -->
An `advised` note migration, ID `20261112-a-draft-s-clm`, SHALL offer a project the two allow rules for `clm setup` that `init/init-seeds-permissions` adds to `clm init`'s seeding, `Bash(clm setup)` and `Bash(clm setup *)`, when the project lacks either. It SHALL hold its own copy of the two rules, since `upgrade` doesn't depend on `init`, and `20261107-projects-set-up-before`, which is frozen once shipped (`shipped-migrations-frozen`), keeps offering the other twenty-two. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds both rules, read and compared as `migration-unattended-allow-rules` reads and compares them. Its `file` SHALL be `.claude/settings.json`, and it SHALL carry no `old` or `new`, so `clm upgrade --apply` never writes the settings (`unattended-steps-run-clm`, `allow-rules-offered-by-note`). Its note's first line SHALL read `Unattended sessions need clm's allow rules for clm setup in .claude/settings.json`. The note SHALL list the two rules; say that builds and drafts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`) and that Claude Code's auto mode denies it when no rule allows it; say that the human adds them to `.claude/settings.json` by hand and that `clm init --check-permissions` prints the rules the project lacks; and say that `clm upgrade --decline 20261112-a-draft-s-clm` keeps a project that leaves them out on purpose as it is. `init/init-seeds-permissions`' build writes this migration (`template-change-ships-its-migration`), and gives the fixtures that stand for an up-to-date project and that this migration would leave pending the two rules: `crates/clm-upgrade/src/shipped_tests.rs`'s `project()`, the `clm_repo` fixture of `crates/clm/tests/hooks.rs`'s `session_start_reminds_to_upgrade`, and the capability test `capability/upgrade-clm`'s *skipping releases*, as `migration-unattended-allow-rules` states for its own rules; and it adds this migration's ID to the allow-rules migrations `crates/clm/tests/upgrade.rs`'s corpus test declines when pending (`clm/upgrade-corpus`), naming that edit and the `clm_repo` one under `## Fixes in other components` in its pull request, since every corpus entry set up before `clm init` seeded the two rules otherwise ends with this migration pending.

#### Scenario: a project lacking the setup rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every rule `20261107-projects-set-up-before` offers but neither `clm setup` rule
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261112-a-draft-s-clm advised note Unattended sessions need clm's allow rules for clm setup in .claude/settings.json` and doesn't list `20261107-projects-set-up-before`, the second prints `note 20261112-a-draft-s-clm`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing `Bash(clm setup)`, and a `.claude/settings.local.json` allowing `Bash(clm setup *)`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-a-draft-s-clm`

#### Scenario: declined
- GIVEN a project lacking the setup rules, whose `clm.toml` sets `declined_migrations = ["20261112-a-draft-s-clm"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-a-draft-s-clm`

#### Needs
- init/init-seeds-permissions
