---
component: plugin
---

## ADDED Requirements

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
