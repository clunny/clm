---
component: verify
---

## MODIFIED Requirements
### Requirement: verify-run <!-- base: 7308e44a4a12d022 -->
In `local` and `pipeline` mode, a due `clm verify --due` SHALL check `origin/main`'s head out into `.clm/verify-tree` in the CLM root of the main worktree and run `[verify] check` there through `land/shared-check-runner`, with `CLM_SELECTION` naming the output of `clm affected --base <last green commit>` when a green record exists and unset otherwise, so under the clone's `.clm/check.lock`, its output copied to stderr and to `.clm/verify-logs/<sha>.log`, removed when the check passes and kept when it fails. It SHALL then append one `verified` record naming the head, the result, the last verified commit as `since`, the unverified lands, the culprit and components (`verify-bisect`), the mode and no `run`, print `verify passed <sha> <n> lands` and exit 0, or print `verify failed <sha> culprit <land|unknown> components <component>...` and the log's path and exit 1. When the check fails, it SHALL then print on stderr, after the outcome line and before the log's path, what `land/check-log`'s refusal holds of the first run of the check on the head, not of a bisect probe: a line naming the check, the last 40 lines of that run's output, and, when a step was open as its output ended (`check-step-markers`), a line naming that step and the first and last line numbers of the lines it spans, counted from 1 in the whole output. The step and the lines SHALL be those the shared check runner returns, and a check that marks no step SHALL get no step line. A run that ends before the check exits SHALL append nothing. A write to stderr that fails SHALL neither end the run nor change its result or exit status before the record is appended, and nothing SHALL be printed to stdout before it, so once the check has exited its record is appended whatever becomes of `clm verify`'s stdout and stderr; what the command prints after the record MAY still end it, as `clm/closed-stdout` says.

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
- THEN stdout holds the `verify failed` line, and stderr, after it, holds a line naming the check, the lines `11` to `50`, a line naming the step `unit` and lines 1 to 51, and the log's path

#### Scenario: no step marked
- GIVEN the same, with a `[verify] check` that prints the lines `1` to `3` and exits 1
- WHEN `clm verify --due` runs with a trigger holding
- THEN stderr holds the three lines after the line naming the check, and no line naming a step

#### Needs
- config/verify-setting
- ledger/verified-record
- land/shared-check-runner

### Requirement: verify-commit <!-- base: ab79a067e8c6fb96 -->
`clm verify --commit <sha>` SHALL take as `<sha>` any revision that `git rev-parse` resolves to a commit once `origin` has been fetched, such as `origin/main`, and SHALL refuse with exit 1 one that resolves to none. It SHALL refuse with exit 1 a commit that isn't on `origin/main`'s first-parent chain. For one that is, it SHALL answer from the ledger when a `verified` record names it, printing `verify passed` or `verify failed` and exiting 0 or 1, and otherwise verify it as `verify-run` or `verify-import` verifies `origin/main`'s head, triggers aside, with the commit checked out and the last verified commit before it as `since`, appending the record.

#### Scenario: already verified
- GIVEN a passing `verified` record for commit `c` on `main`
- WHEN `clm verify --commit c` runs
- THEN it prints `verify passed` naming `c`, exits 0 and runs no check

#### Scenario: verified now
- GIVEN `mode = "local"`, no record for `origin/main`'s head `h`, and a `[verify] check` that exits 0
- WHEN `clm verify --commit h` runs
- THEN the check ran on `h`, a passing record for `h` was appended, and it exits 0

#### Scenario: not on main
- GIVEN a commit only on a branch
- WHEN `clm verify --commit <it>` runs
- THEN it exits 1 saying the commit isn't on `main`, and runs nothing

#### Scenario: main by name
- GIVEN `mode = "local"`, an `origin/main` whose head `h` is one land ahead of the clone's last fetch, no record for `h`, and a `[verify] check` that exits 0
- WHEN `clm verify --commit origin/main` runs
- THEN the check ran on `h`, a passing record for `h` was appended, and it printed `verify passed` naming `h`

#### Needs
- config/verify-setting
- host/list-runs
- ledger/verified-record
