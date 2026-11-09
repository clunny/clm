---
component: plugin.build
---

## ADDED Requirements
### Requirement: build-defect-fixes-run-setup
`skills/build/SKILL.md` SHALL tell Claude, in the step that fixes reported defects in a worktree of its own, that when `clm.toml` sets `[build] setup` it runs `clm setup` in that worktree before it runs any test there, and again after an edit or a `clm rebase` that changes the project's dependency declarations, so the fix's tests run against that worktree's own code and dependencies, never the main checkout's (`tests-run-in-their-own-tree`); and that when `clm setup` fails it fixes nothing there that needs a test, and says so in its summary.

#### Scenario: the shipped build skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN the paragraph that holds `fix reported defects` also holds `clm setup` and `tests-run-in-their-own-tree`

#### Needs
- orchestrator.run/setup-command

#### Serves
- architecture/tests-run-in-their-own-tree
