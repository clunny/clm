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
`clm` SHALL dispatch `clm upgrade [--apply | --note <id> | --done <id>]` to `clm-upgrade`, for the CLM repository found from the current directory (`root-from-cwd`).

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm upgrade` runs in a subdirectory of it
- THEN it prints `up to date` and exits 0

#### Needs
- upgrade/upgrade-list

#### Serves
- capability/upgrade-clm

### Requirement: upgrade-blocks-commands
Inside a CLM repository, `clm` SHALL call `clm-upgrade`'s `require_current` before running any command except `--version`, `help`, `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook` and `view`, and, when it fails, SHALL print its error to stderr and exit 2 without running the command. Outside a CLM repository it SHALL NOT call it. A command added later SHALL be blocked unless it is added to that list.

#### Scenario: required migration pending
- GIVEN a CLM repository with a pending `required` migration
- WHEN `clm check` runs, and then `clm status` and `clm upgrade`
- THEN `clm check` exits 2 naming the migration and `clm upgrade`, and `clm status` and `clm upgrade` run as usual

#### Scenario: upgraded by a newer clm
- GIVEN a CLM repository whose `clm.toml` sets `upgraded` to an ID this binary doesn't hold
- WHEN `clm land` runs
- THEN it exits 2 saying the project needs a newer `clm`

#### Scenario: advised only
- GIVEN a CLM repository with only a pending `advised` migration
- WHEN `clm check` runs
- THEN it runs as usual

#### Needs
- upgrade/require-current

#### Serves
- capability/upgrade-clm
