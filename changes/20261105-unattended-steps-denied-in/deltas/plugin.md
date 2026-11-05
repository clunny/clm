---
component: plugin
---

## ADDED Requirements

### Requirement: unattended-permissions-check
`skills/build/SKILL.md` and `skills/triage/SKILL.md` SHALL each tell Claude, once, before the first build or draft it launches, to run `clm init --check-permissions` and, when it exits 1, to tell the human that in auto mode Claude Code may deny the unattended steps those rules allow, show the rules it printed, say that they belong in `permissions.allow` in `.claude/settings.json` (or `.claude/settings.local.json` for this clone alone), and go on launching, never refusing to; when it exits with any other code, as a `clm` without the option does, to tell the human the check couldn't run and go on launching; and never to write either file itself. `skills/build/SKILL.md` SHALL tell Claude to push a branch it rebased with `clm branch push` and to release a claim it took with `clm branch delete`, and SHALL NOT contain `git push --force-with-lease`.

#### Scenario: skills check the rules
- GIVEN the plugin's `skills/build/SKILL.md` and `skills/triage/SKILL.md`
- WHEN a test reads them
- THEN each contains `clm init --check-permissions`, `auto mode` and `.claude/settings.json`, and the build skill contains `clm branch push` and `clm branch delete` and doesn't contain `git push --force-with-lease`

#### Scenario: skill without the check
- GIVEN a `triage` `SKILL.md` that doesn't mention `clm init --check-permissions`
- WHEN the test's check runs on it
- THEN it reports the skill and `clm init --check-permissions`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- init/init-check-permissions
- host/branch-push
- host/branch-delete
