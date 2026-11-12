---
component: clm
---

## MODIFIED Requirements

### Requirement: claude-md-pushes-with-clm <!-- base: 78576ed9470432e9 -->
This repository's instructions, its `CLAUDE.md` and, when it exists, the `AGENTS.md` that `CLAUDE.md` imports (`init`'s layout, which `upgrade/migration-agents-md` moves a project to), SHALL NOT contain `git push`. Wherever they tell a session to push a branch to `origin` they SHALL name `clm branch push`, as the skills do (`plugin/skills-push-with-clm`), so a session in this repository that follows them under Claude Code's auto mode runs only `clm` commands to write to `origin` (`unattended-steps-run-clm`). The rule holds whichever of the two files holds the instructions, before and after the migration, so taking `migration-agents-md` here keeps it true. Its test reads both files and holds a `clm:reads` marker for each (`doc-readers-marked`).

#### Scenario: CLAUDE.md alone pushes through clm
- GIVEN the repository's `CLAUDE.md` holding the instructions and no `AGENTS.md`
- WHEN a test reads them
- THEN `CLAUDE.md` doesn't contain `git push`, and it contains `clm branch push`

#### Scenario: AGENTS.md holds the instructions
- GIVEN a `CLAUDE.md` of the one line `@AGENTS.md` and an `AGENTS.md` holding the instructions
- WHEN a test reads them
- THEN neither file contains `git push`, and their texts together contain `clm branch push`

#### Scenario: a git push in either file
- GIVEN a `CLAUDE.md` and an `AGENTS.md`, one of which contains `git push`
- WHEN the test's check runs on their texts
- THEN it fails, naming that file

#### Serves
- capability/guard-sessions

#### Needs
- host/branch-push
