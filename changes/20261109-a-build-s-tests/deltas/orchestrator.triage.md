---
component: orchestrator.triage
---

## ADDED Requirements
### Requirement: draft-prompt-setup
When `clm.toml` sets `[build] setup`, the prompt `clm draft` writes SHALL tell the session, in the paragraph that describes fixing code-only bugs by component, that once it is on the branch it fixes code on (the `impl/<component>` branch `clm claim <component> --fix` switched it to, or a branch of its own for test plumbing) and before it runs any test there, it runs `clm setup` as one plain command, which runs the project's `[build] setup` so that its tests run against its own worktree's code and dependencies, never the main checkout's (`tests-run-in-their-own-tree`), and runs it again after a `clm rebase` or an edit of its own that changes the project's dependency declarations; and that a `clm setup` that fails, or that Claude Code refuses or denies, leaves the item it was fixing with `left <slug>: setup failed`, reported with `clm defect` when it was refused or denied, since no test result is then to be trusted. When `clm.toml` sets no `[build] setup`, the prompt SHALL NOT name `clm setup`. `clm draft` SHALL take the setting from the settings `crates/clm` loads and passes it, as it passes `max_builds`, so `clm-triage` reads no settings itself. The text SHALL keep to the wording `draft` allows.

#### Scenario: setup declared
- GIVEN a repository whose `clm.toml` sets `[build] setup = "scripts/setup.sh"`, and a backlog item `b`
- WHEN `clm draft b` writes its prompt
- THEN the paragraph of the prompt file that contains `code-fix-needs-a-written-rule` also contains `clm setup`, `tests-run-in-their-own-tree` and `setup failed`

#### Scenario: nothing declared
- GIVEN a repository whose `clm.toml` has no `[build]` table, and a backlog item `b`
- WHEN `clm draft b` writes its prompt
- THEN the prompt file doesn't contain `clm setup`

#### Needs
- config/build-setup-setting

#### Serves
- architecture/tests-run-in-their-own-tree
