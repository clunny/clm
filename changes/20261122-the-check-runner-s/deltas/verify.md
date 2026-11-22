---
component: verify
---

## MODIFIED Requirements

### Requirement: verify-run <!-- base: 335c90bbd8d417a5 -->
In `local` and `pipeline` mode, a due `clm verify --due` SHALL check `origin/main`'s head out into `.clm/verify-tree` in the CLM root of the main worktree and run `[verify] check` there through `land/shared-check-runner`, with `CLM_SELECTION` naming the output of `clm affected --base <last green commit>` when a green record exists and unset otherwise, so under the clone's `.clm/check.lock`, its output copied to stderr and to `.clm/verify-logs/<sha>.log`, removed when the check passes and kept when it fails. Every run of `[verify] check` `clm verify` makes, here, under `--commit` and in each bisect probe (`verify-bisect`, `--bisect` included), SHALL give the runner the name `clm verify`, so when the log can't be created the runner's warning reads `clm verify: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone`. `clm-verify`'s code and its tests SHALL call the runner only in the form that takes a name (`land/shared-check-runner`), a test that stands in for a land's check giving `clm land`. Before a checkout, a verify tree that git doesn't list as a worktree, that isn't whole (no `.git` file at its top, or git run there finding its top elsewhere), or in which the checkout fails SHALL be removed, whatever it is, and added again as a worktree of the clone at the commit. When the selection can't be made, it SHALL print `clm affected failed: <why>; the check runs every test` on stderr and run the check with `CLM_SELECTION` unset, the run going on as any other. It SHALL then append one `verified` record naming the head, the result, the last verified commit as `since`, the unverified lands, the culprit and components (`verify-bisect`), the mode and no `run`, print `verify passed <sha> <n> lands` and exit 0, or print `verify failed <sha> culprit <land|unknown> components <component>...` and the log's path and exit 1. When `ledger`'s append leaves a `verified` record that `clm verify` appends committed on the local ledger branch and unpushed (`ledger/append-refused-push`), here or in `verify-import` or `verify-bisect`, its notes on stderr SHALL hold ``the record is committed locally; `clm reconnect` pushes it``, the outcome line and exit status unchanged. When the check fails, it SHALL then print on stderr, after its other notes and just before the log's path, what `land/check-log`'s refusal holds of the run of the check on the commit it verifies (the head, or the commit `--commit` names), not of a bisect probe: the line ``last 40 lines of `<check>`:``, the last 40 lines of that run's output, and, when a step was open as its output ended (`land/check-log`), the line ``the step open when it ended is `<name>`, lines <first> to <last> of its output``, counted from 1 in the whole output. The step and its lines SHALL be as `land/check-log` names and counts them, and a check that marks no step SHALL get no step line. A result answered from the ledger, a `--bisect` search and an `import` result run no check there and SHALL print no such report. A run that ends before the check exits SHALL append nothing. A write to stderr that fails SHALL neither end the run nor change its result or exit status before the record is appended, and nothing SHALL be printed to stdout before it, so once the check has exited its record is appended whatever becomes of `clm verify`'s stdout and stderr; what the command prints after the record MAY still end it, as `clm/closed-stdout` says.

#### Scenario: passing run
- GIVEN `mode = "local"`, `triggers = []`, `every = { lands = 1 }`, one land since a passing record, and a `[verify] check` that records `CLM_SELECTION` and its working directory and exits 0
- WHEN `clm verify --due` runs
- THEN the check ran in `.clm/verify-tree` on `origin/main`'s head with `CLM_SELECTION` set, the ledger holds a `verified` record for the head with `passed: true` and that one land, and it printed `verify passed <sha> 1 lands` and exited 0

#### Scenario: failing run
- GIVEN the same settings and a `[verify] check` that exits 1
- WHEN `clm verify --due` runs
- THEN it exits 1, prints `verify failed <sha> culprit <the land>` with that land's components and the log's path, and the ledger holds a `verified` record with `passed: false`

#### Scenario: a land waits for the slow check
- GIVEN a `[verify] check` that appends `start verify` to a log, sleeps 2 seconds and appends `end verify`, and a code branch whose `[land] check` appends `land` to the log
- WHEN `clm verify --due` starts and `clm land` runs on the branch while it sleeps
- THEN the log reads `start verify`, `end verify`, `land`, and the land printed `waiting for another check` naming `.clm/check.lock`

#### Scenario: one at a time
- GIVEN a `[verify] check` that appends `start` to a log, sleeps 2 seconds and appends `end`
- WHEN two `clm verify --due` start together in one clone
- THEN the log reads `start`, `end`, `start`, `end`, and one printed `waiting for another check`

#### Scenario: the verdict outlives a failing stderr
- GIVEN Linux, `mode = "local"`, two lands since a passing record, the first changing a file of component `a`, a `[verify] check` that prints 1000 lines and fails on any tree holding the first land's change, `.clm/check.lock` held by another process that releases it after 1 second, and `clm verify`'s stderr opened on `/dev/full`
- WHEN `clm verify --due` runs with a trigger holding
- THEN the ledger holds a `verified` record for the head with `passed: false`, culprit the first land and components `a`, and stdout holds `verify failed <sha> culprit <the first land> components a`, whatever its exit status

#### Scenario: a red run's report
- GIVEN `mode = "local"`, one land since a passing record, and a `[verify] check` that prints `::group::unit`, the lines `1` to `50` and exits 1
- WHEN `clm verify --due` runs with a trigger holding
- THEN stdout holds the `verify failed` line, and stderr, after it, holds ``last 40 lines of `<check>`:``, the lines `11` to `50`, ``the step open when it ended is `unit`, lines 1 to 51 of its output`` and the log's path, in that order

#### Scenario: no step marked
- GIVEN the same, with a `[verify] check` that prints the lines `1` to `3` and exits 1
- WHEN `clm verify --due` runs with a trigger holding
- THEN stderr holds the three lines after ``last 40 lines of `<check>`:``, and no line starting `the step open`

#### Scenario: a broken tree is remade
- GIVEN `mode = "local"`, `every = { lands = 1 }`, a passing record, and in turn: a verify tree from an earlier run whose `.git` file at its top was removed; and a regular file at the main worktree's `.clm/verify-tree`
- WHEN, in each case, one land follows and `clm verify --due` runs
- THEN the check ran in `.clm/verify-tree` on `origin/main`'s head, it printed `verify passed <sha> 1 lands` and exited 0, and git lists `.clm/verify-tree` as a worktree

#### Scenario: a selection that cannot be made
- GIVEN `mode = "local"`, one land since a passing record that adds `specs/b/SPEC.md` holding text that isn't a spec, and a `[verify] check` that records whether `CLM_SELECTION` is set and exits 0
- WHEN `clm verify --due` runs with a trigger holding
- THEN the check ran with `CLM_SELECTION` unset, stderr holds a line starting `clm affected failed: ` and ending `; the check runs every test`, the ledger holds a `verified` record with `passed: true`, and it printed `verify passed` and exited 0

#### Scenario: a record kept local
- GIVEN `mode = "local"`, one land since a passing record, a `[verify] check` that exits 0, and an `origin` whose `pre-receive` hook refuses every push to `refs/heads/clm/ledger/*`
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify passed <sha> 1 lands` and exits 0, stderr holds ``the record is committed locally; `clm reconnect` pushes it``, and the clone's own ledger branch holds the `verified` record

#### Scenario: the log's warning names clm verify
- GIVEN `mode = "local"`, one land since a passing record, a `[verify] check` that exits 0, and `.clm/verify-logs` a file, so the check's log can't be created
- WHEN `clm verify --due` runs with a trigger holding
- THEN stderr holds a line starting `clm verify: warning: can't write the check's log at ` and none starting `clm land:`, and it prints `verify passed <sha> 1 lands` and exits 0

#### Needs
- config/verify-setting
- ledger/verified-record
- land/shared-check-runner
