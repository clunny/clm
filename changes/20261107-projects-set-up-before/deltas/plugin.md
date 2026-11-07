---
component: plugin
---

## ADDED Requirements

### Requirement: upgrade-skill-settings-by-hand
`skills/upgrade/SKILL.md` SHALL tell Claude, for the note on `.claude/settings.json` (`upgrade/migration-unattended-allow-rules`), never to edit that file itself, since it holds the human's settings and is a protected path (`unattended-steps-run-clm`): it shows the human the rules `clm init --check-permissions` prints in the upgrade's worktree, asks them to add those they want to `.claude/settings.json` there by hand, and runs `clm upgrade --decline` for the migration when they want none. When the root component's `paths` don't list `.claude/settings.json` and git doesn't track it yet, as in a project set up before `clm init` seeded the rules, the skill SHALL tell Claude to put the file's addition to the root's `paths` on the notes branch, as a change of its own as `/clm:propose` makes one (`changes/<id>/` with a delta for the root's frontmatter, applied), landing it as that branch lands, and to commit the settings file only after that, on a third branch `upgrade/<YYYYMMDD>-settings` cut from the new `origin/main` and landed as the code branch is, since `.claude/settings.json` is a code path and `clm check` reports a tracked file that no component's `paths` hold. A project whose root already lists the file takes the human's settings edit on the first, code, branch.

#### Scenario: the shipped upgrade skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN a test reads it
- THEN it contains `.claude/settings.json`, `clm init --check-permissions` and `by hand`

#### Needs
- upgrade/migration-unattended-allow-rules
