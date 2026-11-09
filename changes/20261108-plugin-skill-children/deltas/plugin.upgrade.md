---
component: plugin.upgrade
create: true
---

## Frontmatter
```yaml
title: Upgrade skill
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["plugin/skills/upgrade/**"]
successor: ""
serves: [architecture/method-in-the-plugin]
```

## Contract
The skill `upgrade`, `plugin/skills/upgrade/SKILL.md`, which brings a project up to date with the `clm` it runs: it takes the migrations `clm upgrade` lists and lands them (`upgrades-carry-migrations`). The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file.

## Design
The skill only runs `clm upgrade`; which migrations exist, what each checks and what each writes are `upgrade`'s (`hook-logic-in-clm`).

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_upgrade.rs`, a test file of its own beside `plugin.rs`: its build moves each test that cites one of its requirements out of `plugin.rs` into that file, with copies of the helpers those tests use and the `clm:reads` markers for the files they read (`doc-readers-marked`), so its code count holds its own tests rather than the whole of `plugin.rs`. `plugin`'s tests of the shared rules read this component's file too; a build that changes the file keeps them passing.

## ADDED Requirements
### Requirement: upgrade-skill
The plugin SHALL ship `skills/upgrade/SKILL.md`, with a `description` in its frontmatter saying it brings a project up to date with the `clm` it runs. It SHALL tell Claude to:
- run `clm upgrade` and, when it prints `up to date`, say so and stop;
- cut a branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own (`session-worktrees`), and work there;
- run `clm upgrade --apply`;
- for each `note <id>` it prints, read the note with `clm upgrade --note <id>`, show the human the edit it proposes beside the file's current text, and make it on their yes, or run `clm upgrade --decline <id>` on their no, an edit to `ARCHITECTURE.md` or `CAPABILITIES.md` being one the human then approves again with `clm approve`;
- when `clm upgrade` still lists a `required` migration after `--apply`, or a migration fails, report it with `clm defect`, naming `upgrade` with `--component`, and tell the human that `skip_migrations` in the clone's `clm.local.toml` lets this clone run meanwhile, never setting it without their yes;
- run `clm check`; commit the changes outside spec paths on the branch and land it when there are any, since mechanical migrations are code and a note's edit is spec, which `clm gate` refuses together as `mixed`; then commit the rest, with `clm.toml`'s `upgraded` and `declined_migrations` lines, on a branch `upgrade/<YYYYMMDD>-notes` cut from the new `origin/main`, and land it; landing each by the first line of `clm gate --base origin/main`, as `propose` lands a branch, asking for the human's `clm approve` when it prints `spec review`.

#### Scenario: the skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN it is read
- THEN it names `clm upgrade --apply`, `clm upgrade --note`, `clm upgrade --decline`, `clm defect`, `skip_migrations`, `mixed` and `clm land`

#### Needs
- clm/upgrade-command
- gate/upgrade-stamp-left-out
- config/upgrade-settings

#### Serves
- capability/upgrade-clm

### Requirement: upgrade-skill-offers-verify
`skills/upgrade/SKILL.md` SHALL tell Claude, when `clm upgrade --apply` prints `applied` for the migration that adds the commented `[verify]` section (`upgrade/migration-offer-verify`), to ask the human the same questions `/clm:init` asks about slow tests (`init-skill-asks-verify`), set `[verify]` on a yes, and leave the section commented on a no.

#### Scenario: the shipped upgrade skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN a test reads it
- THEN it contains `[verify]` and `Slow tests can run after the land`

#### Needs
- upgrade/migration-offer-verify
- clm/verify-command

#### Serves
- capability/slow-tests-after-land

### Requirement: upgrade-skill-settings-by-hand
`skills/upgrade/SKILL.md` SHALL tell Claude, for the note on `.claude/settings.json` (`upgrade/migration-unattended-allow-rules`), never to edit that file itself, since it holds the human's settings and is a protected path (`unattended-steps-run-clm`): it shows the human the rules `clm init --check-permissions` prints in the upgrade's worktree, asks them to add those they want to `.claude/settings.json` there by hand, and runs `clm upgrade --decline` for the migration when they want none. When the root component's `paths` don't list `.claude/settings.json` and git doesn't track it yet, as in a project set up before `clm init` seeded the rules, the skill SHALL tell Claude to put the file's addition to the root's `paths` on the notes branch, as a change of its own as `/clm:propose` makes one (`changes/<id>/` with a delta for the root's frontmatter, applied), landing it as that branch lands, and to commit the settings file only after that, on a third branch `upgrade/<YYYYMMDD>-settings` cut from the new `origin/main` and landed as the code branch is, since `.claude/settings.json` is a code path and `clm check` reports a tracked file that no component's `paths` hold. A project whose root already lists the file takes the human's settings edit on the first, code, branch.

#### Scenario: the shipped upgrade skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN a test reads it
- THEN it contains `.claude/settings.json`, `clm init --check-permissions` and `by hand`

#### Needs
- upgrade/migration-unattended-allow-rules

### Requirement: upgrade-skill-notes-without-edit
`skills/upgrade/SKILL.md` SHALL tell Claude that a `note <id>` whose note says it edits no file, such as the one telling a project its tests must now overlap (`upgrade/migration-tests-overlap`), asks for no edit: Claude shows the human the note, offers to do what it advises (such as looking through the check script and tests for fixed ports, paths and shared servers and recording each as a `bug` backlog item on the notes branch) and does it on their yes, and runs no `clm upgrade --decline` for it, since `clm upgrade --apply`'s stamp has already taken it (`stamp-taken-notes`).

#### Scenario: the shipped upgrade skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN a test reads it
- THEN it contains `edits no file` and `stamp-taken-notes`

#### Needs
- upgrade/migration-tests-overlap

#### Serves
- capability/upgrade-clm
