---
component: clm
---

## MODIFIED Requirements

### Requirement: routes-only <!-- base: 0b2d8731c8e37213 -->
For every command a component offers as a call (`commands-in-crate`), `crates/clm` SHALL only parse the command line with the crate's own argument definitions, apply what holds for every command (`upgrade-blocks-commands`, `none-remote-before-commands`, `root-from-cwd`, `closed-stdout`), load the settings the crate is passed, and call the crate with the process's standard output, standard error and environment: it SHALL print nothing of its own for such a command and SHALL exit with the status the call returns. The top-level test files under `crates/clm/tests/` SHALL hold tests of the root's own requirements only, and the helpers no test there uses are removed with the tests that moved (`commands-live-in-their-crates`).

#### Scenario: a routed command
- GIVEN a fixture repository
- WHEN `clm hashes <component>` runs as a process there, and a test calls `clm-apply`'s call with the same arguments and two buffers
- THEN the process's standard output and standard error equal the buffers, and its exit status is what the call returned

#### Scenario: a status the call returns
- GIVEN a fixture repository whose check fails
- WHEN `clm check` runs as a process
- THEN it exits with the status `clm-check`'s call returns for that repository, and prints nothing the call didn't write

#### Scenario: the settings a routed command is passed
- GIVEN a fixture repository whose `clm.toml` sets `[affected]` `all = ["Cargo.lock"]`, on a branch that changes only `Cargo.lock`, and the same repository with `all = 3`
- WHEN `clm affected --base main` runs as a process in each
- THEN the first prints only `all Cargo.lock`, and the second exits non-zero with a line naming `clm.toml` and `check.affected/affected-all`, as `clm check` prints a violation

#### Scenario: the budgets a routed command is passed
- GIVEN a fixture repository whose `clm.toml` sets `[component_size]` `code_tokens = 100`, with component `a` owning a tracked file of 200 tokens and component `b` within its budgets
- WHEN `clm status` runs as a process
- THEN `a`'s row holds the cell `oversized` and `b`'s doesn't

#### Scenario: every command routes
- GIVEN the commands `clm --help` lists
- WHEN each runs as a process with `--help`
- THEN each exits 0 and prints its own usage

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- apply/commands-in-crate
- check/commands-in-crate
- check.affected/commands-in-crate
- check.health/commands-in-crate
- check.unreferenced/commands-in-crate
- config/commands-in-crate
- context/commands-in-crate
- effort/commands-in-crate
- gate/commands-in-crate
- host/commands-in-crate
- init/commands-in-crate
- land/commands-in-crate
- land.approval/commands-in-crate
- ledger/commands-in-crate
- orchestrator/commands-in-crate
- orchestrator.checkpoint/commands-in-crate
- orchestrator.run/commands-in-crate
- orchestrator.triage/commands-in-crate
- plugin.hooks/commands-in-crate
- plugin.hooks.guard/commands-in-crate
- rename/commands-in-crate
- sync/commands-in-crate
- view.repo/commands-in-crate
- view.repo/reads-the-repository
- view.repo/arguments-and-exit
- view/status-report
- view/view-command
- view/view-library
- check.coverage/pending-report

