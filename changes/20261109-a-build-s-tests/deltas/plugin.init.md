---
component: plugin.init
---

## ADDED Requirements
### Requirement: init-skill-build-setup
`skills/init/SKILL.md` SHALL, once `clm init` has written `clm.toml`, tell Claude to tell the human that each CLM build runs in a fresh worktree inside the main checkout with nothing installed, and that its tests must run against that worktree's own code and dependencies, never the main checkout's (`tests-run-in-their-own-tree`): a toolchain whose lookup walks up the parent directories, or an environment the checkouts share, would otherwise find the main checkout's packages or source, so the project declares in `clm.toml`'s `[build] setup` the command that makes a fresh worktree ready to run its tests, which `clm setup` runs before a build's first test run. For a repository with code, it SHALL tell Claude to work out, from the project's toolchains and how its tests find their dependencies, whether a fresh worktree needs such a command, and to propose one for the human to confirm, writing it into `clm.toml` on the first spec branch when they agree.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `tests-run-in-their-own-tree`, `[build] setup`, `clm setup` and `fresh worktree`

#### Serves
- architecture/tests-run-in-their-own-tree
