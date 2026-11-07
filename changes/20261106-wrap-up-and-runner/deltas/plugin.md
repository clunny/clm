---
component: plugin
---

## ADDED Requirements

### Requirement: wrap-up-left-lines
Each of `skills/build/SKILL.md`, `skills/triage/SKILL.md`, `skills/propose/SKILL.md`, `skills/contribution/SKILL.md` and `skills/init/SKILL.md` that tells Claude to run `clm wrap-up` SHALL tell Claude to pass each `left <path>: ...` line `clm wrap-up` prints to the human as it stands, a build's worktree it couldn't remove, which doesn't make the wrap-up incomplete, and to leave the directory, and any command the line names, to them. No skill SHALL tell Claude to run `git worktree prune` or delete such a directory itself.

#### Needs
- orchestrator.checkpoint/checkpoint

#### Scenario: wrap-up's left lines passed on
- GIVEN the plugin's skills
- WHEN a test reads each one that names `clm wrap-up`
- THEN it holds a paragraph that names both `clm wrap-up` and `left <path>`, and no skill holds `git worktree prune`

#### Scenario: skill without the line
- GIVEN a `SKILL.md` that names `clm wrap-up` and no paragraph naming `left <path>` with it
- WHEN the test's check runs on it
- THEN it reports that skill
