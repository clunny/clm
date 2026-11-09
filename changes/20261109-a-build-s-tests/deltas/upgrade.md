---
component: upgrade
---

## ADDED Requirements
### Requirement: migration-build-setup
An `advised` note migration, ID `20261109-a-build-s-tests`, SHALL tell a project set up before it that a build's tests must run against its own worktree's code and dependencies, and how to declare the setup that makes them. It edits no file: its `file` SHALL be `clm.toml`, where `[build] setup` goes, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()`, and false otherwise, as `migration-tests-overlap`'s is. The fixture `migration-tests-overlap` names for an up-to-date project SHALL stay stamped with `latest()`. A test that expects a project stamped before an earlier note to list only that note, such as `migration-land-dir`'s, SHALL expect this one beside it, and `migration-starter-land-dir` (`20261109-land-still-sets-cargo`) too when the fixture holds a `scripts/clm-check.sh` that runs `cargo` and names no `CLM_LAND_DIR`. Its note's first line SHALL read `Builds run your tests in a fresh worktree: declare [build] setup`. The note SHALL say:
- that each build runs in a fresh worktree inside the main checkout, with nothing installed, so a toolchain whose lookup walks up the parent directories, or an environment the checkouts share, can resolve the main checkout's packages or source instead of the branch's, and a build's red or green test then means nothing (`tests-run-in-their-own-tree`, which `clm guide constraints` states);
- that the project declares in `clm.toml` `[build] setup`, a shell command that makes a fresh worktree ready to run its tests, such as one installing its dependencies into it, and that `clm setup` runs it, which builds and drafts do before their first test run, while `[land] check` stays a command that needs no setup before it;
- how to check the project: create a worktree under the main checkout, run its tests there without installing anything, and see whether they fail for want of their dependencies or resolve them from outside the worktree; for example, a project whose tests use npm packages can set `setup = "test ! -f package.json || npm ci"`;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done (its fixture holds no `scripts/clm-check.sh`, so `migration-starter-land-dir` is done)
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261109-a-build-s-tests advised note Builds run your tests in a fresh worktree: declare [build] setup`, the second prints `note 20261109-a-build-s-tests` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261109-a-build-s-tests` runs on a project it is pending in
- THEN the note names `tests-run-in-their-own-tree`, `clm guide constraints`, `[build] setup`, `clm setup` and `[land] check`

#### Needs
- clm/upgrade-command
- config/build-setup-setting
- orchestrator.run/setup-command

#### Serves
- architecture/tests-run-in-their-own-tree
