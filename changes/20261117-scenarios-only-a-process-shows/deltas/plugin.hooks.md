---
component: plugin.hooks
base: { design: "86c120ecb3081d86" }
---

## MODIFIED Design

Crate `clm-hooks`, called from `crates/clm`'s `clm hook session-start`, `clm hook autosync` and `clm defect` subcommands. The guard is crate `clm-guard`, the child `plugin.hooks.guard`, with the shell reading as its child `plugin.hooks.guard.shell` (`guard-split`); `clm-guard` finds the CLM root with this crate's `clm_root` and loads settings there under its call's own environment lookup, where this crate's `settings` reads the process's, and this crate depends on nothing of it.

This component's tests are under `crates/clm-hooks/tests/` and call the crate (`commands-in-crate`); that `clm check` accepts an item `clm defect` wrote is the root's to prove (`clm/check-across-components`).

**Defects** (`defect-reports`). `clm defect --title <title> [--component <component>]...` reads the report from stdin and loads the settings for the CLM repository found from the current directory, or none. By `defects`:
- `backlog` (`defect-backlog`): `clm check` validates the components as for any item.
- `ask` (`defect-issue`, `defects-file-on-github`): the issue's body is the report under `## What happened`, then the components, the `clm` version and the OS. It goes through `clm_host::adapter(Host::Github)`, never the adapter of the `host` setting. A report it can't file is kept in `clm/defects/` under the directory `clm_git::common_dir` gives for the current directory, named by the same slug function and `create_new` loop as a backlog report, so two reports with one title at once can't overwrite each other.
- `off`: reporting is off.



## MODIFIED Requirements



### Requirement: defect-backlog <!-- base: 064ef91e3ad01881 -->
With `defects` set to `backlog`, `clm defect --title <title> [--component <component>]...` SHALL write `backlog/<slug>.md` with the title, `kind: bug`, the components under the key `components` (`model/component-keys`), the report from stdin as its body, and `origin: <change>` when the current branch is `spec/<change>` and the repository has a live change `<change>`, and print its path. Any other branch, `impl/*` ones included, SHALL give no `origin`. With no CLM repository, or a component a backlog item may not name (`model/backlog-components`: neither in its spec graph nor created by a `create: true` delta of a change outside the archive), it SHALL write nothing and exit 1. The slug SHALL be the title in kebab case, with `-2`, `-3`... added when that file exists.

#### Serves
- capability/report-defects

#### Needs
- model/backlog-components
- model/component-keys
- clm/component-flags

#### Scenario: on a spec branch
- GIVEN a CLM repository with `defects = "backlog"` and a live change `20261022-x`, on branch `spec/20261022-x`
- WHEN `clm defect --title "Gate passes a stale task" --component gate` runs with a report on stdin
- THEN `backlog/gate-passes-a-stale-task.md` has `kind: bug`, `components: [gate]`, `origin: 20261022-x` and the report

#### Scenario: on an implementation branch
- GIVEN the same repository, on branch `impl/gate`, then on `impl/archive/20261022-x`, then on `impl/20261022-x/gate`
- WHEN `clm defect --title "x"` runs on each
- THEN each item it writes has no `origin`

#### Scenario: node a live change creates
- GIVEN the same repository, with a live change whose delta for component `fresh` has `create: true`
- WHEN `clm defect --title "Fresh misfires" --component fresh` runs with a report on stdin
- THEN it writes `backlog/fresh-misfires.md` with `components: [fresh]` and exits 0

#### Scenario: unknown node
- GIVEN the same repository
- WHEN `clm defect --title "x" --component nonesuch` runs
- THEN it writes nothing and exits 1 naming `nonesuch`

#### Scenario: not a CLM repository
- GIVEN a directory with no `specs/SPEC.md` and `CLM_DEFECTS=backlog`
- WHEN `clm defect --title "x"` runs
- THEN it writes nothing and exits 1

#### Scenario: taken slug
- GIVEN the same repository, where `backlog/gate-passes-a-stale-task.md` exists
- WHEN the same `clm defect` runs
- THEN it writes `backlog/gate-passes-a-stale-task-2.md`
