---
component: plugin.propose
---

## ADDED Requirements
### Requirement: propose-fix-runs-setup
`skills/propose/SKILL.md` SHALL tell Claude, where it describes fixing a bug on a code branch test first, that when `clm.toml` sets `[build] setup` it runs `clm setup` in the checkout it fixes in before it runs any test there, and again after an edit or a `clm rebase` that changes the project's dependency declarations, so the fix's tests run against that checkout's own code and dependencies, never another checkout's (`tests-run-in-their-own-tree`).

#### Scenario: the shipped propose skill
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm setup` and `tests-run-in-their-own-tree`

#### Needs
- orchestrator.run/setup-command

#### Serves
- architecture/tests-run-in-their-own-tree
