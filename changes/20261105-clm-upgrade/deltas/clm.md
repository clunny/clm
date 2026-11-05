---
component: clm
base: { frontmatter: "f7285cdd6ff3ca05" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, check, check.affected, check.unreferenced, check.health, gate, apply, context, vcs, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, orchestrator.runner, config, init, view, view.repo, plugin.hooks, plugin.hooks.guard, ledger, land, host, sync, effort, rename, upgrade]
```

## ADDED Requirements

### Requirement: upgrade-command
`clm` SHALL dispatch `clm upgrade [--apply | --note <id> | --decline <id> | --lock <version>]` to `clm-upgrade`, for the CLM repository found from the current directory (`root-from-cwd`).

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm upgrade` runs in a subdirectory of it
- THEN it prints `up to date` and exits 0

#### Needs
- upgrade/upgrade-list
- upgrade/upgrade-apply
- upgrade/upgrade-note-and-decline
- upgrade/shipped-migrations-frozen
- init/init-stamps-upgraded

#### Serves
- capability/upgrade-clm

### Requirement: upgrade-blocks-commands
`clm` SHALL call `clm-upgrade`'s `require_current` before running any command except `--version`, `help`, `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook` and `view`, for the command's `--root` when it takes one and is given it, and otherwise for the CLM repository found from the current directory (`root-from-cwd`); outside any CLM repository it SHALL NOT call it. When the call fails, `clm` SHALL print its error to stderr and exit 4 without running the command. A command added later SHALL be blocked unless it is added to that list.

#### Scenario: required migration pending
- GIVEN a CLM repository with a pending `required` migration
- WHEN `clm check` runs, then `clm status` and `clm upgrade`
- THEN `clm check` exits 4, naming the migration and `clm upgrade`, and `clm status` and `clm upgrade` run as usual

#### Scenario: upgraded by a newer clm
- GIVEN a CLM repository whose `clm.toml` sets `upgraded` to an ID this binary doesn't hold
- WHEN `clm land` runs
- THEN it exits 4, saying the project needs a newer `clm`

#### Scenario: advised only
- GIVEN a CLM repository with only a pending `advised` migration
- WHEN `clm check` runs
- THEN it runs as usual

#### Needs
- upgrade/require-current

#### Serves
- capability/upgrade-clm

### Requirement: upgrade-corpus
For every entry under `crates/clm-upgrade/corpus/`, a copy upgraded with the built `clm upgrade --apply` SHALL be a project where `clm upgrade` prints `up to date`, `clm check` passes, and each file `clm init --lang rust --no-detectors --host none` writes equals what the built `clm init` writes with those flags, `clm.toml`'s `upgraded` line aside. The test SHALL take the expected files from a function it is given, so a test can show that a template changed without a migration fails it. It lives in `crates/clm/tests/upgrade.rs`, since it runs the built binary.

#### Scenario: every release upgrades
- GIVEN the corpus entries `v0.14.0` to the latest release
- WHEN each is upgraded so
- THEN each ends `up to date` and passes `clm check`

#### Scenario: a template changed without a migration
- GIVEN the corpus, and an expected `CLAUDE.md` with one word changed
- WHEN the test's check runs against it
- THEN it fails, naming `CLAUDE.md` and the corpus entry

#### Needs
- upgrade/upgrade-apply
- upgrade/corpus-seeded
- upgrade/migration-carried-constraints-copy
- upgrade/migration-claude-md-names-help
- config/upgrade-settings
- init/init-stamps-upgraded

#### Serves
- architecture/upgrades-carry-migrations
- capability/upgrade-clm
