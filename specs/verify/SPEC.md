---
title: Verify
status: active
owners: ["@clunny"]
depends_on: [config, ledger, host, vcs, model, check.coverage, check.affected, land]
paths: ["crates/clm-verify/**"]
serves: [capability/slow-tests-after-land]
---

## Contract
Running a project's slow tests after the land, at the moments the project chooses, and recording the result in the ledger (`verify-after-land`). A project that sets no `[verify]` (`config`) keeps its slow tests in `[land] check`, and every `clm verify` then prints `verify off: no [verify] in clm.toml` and exits 0. Crate `clm-verify` offers, dispatched by `crates/clm`:
- `clm verify --due [--event work-done] [--dry-run]`: whether a trigger has fired since the last verified commit of `origin/main` (`verify-due`), and when one has, the verification of `origin/main`'s head: in `local` and `pipeline` mode by running `[verify] check` (`verify-run`), in `import` mode by reading the pipeline's result (`verify-import`). With `--dry-run` it only says whether a run is due.
- `clm verify --commit <sha>`: the verification of one commit of `main`, for a release or deploy step (`verify-commit`).
- `clm verify --import`: in `import` mode, the record of every completed pipeline run on `main` newer than the last record, run nothing; in the other modes it prints `verify nothing to import` and exits 0.
- `clm verify --bisect <sha>`: the land that broke a red commit, found by running `[verify] check` on the lands it covers (`verify-bisect`).

Every outcome is one line on stdout starting `verify `: `verify not-due <why>`, `verify due <trigger>`, `verify passed <sha> <n> lands`, `verify failed <sha> culprit <land|unknown> components <component>...`, `verify pending <sha>` and `verify nothing to import`, `<sha>` abbreviated as `git rev-parse --short` abbreviates it. It exits 0 when it passed, nothing was due or there was nothing to import; 1 when the verification failed, or with the reason when it can't run (a `<sha>` not on `origin/main`'s first parents, a `[verify]` it can't use); 2 when git fails; 3 when, with `--dry-run`, nothing is due; and 4 when the result is pending (`import`). Each verification it decides appends one `verified` record (`ledger`); nothing else writes one.

## Design
**Since when.** The *last verified commit* is the newest commit on `origin/main`'s first-parent chain that a `verified` record names, whatever its result; the *last green commit* the newest that one with `passed: true` names. The *unverified lands* are the first-parent commits of `origin/main` after the last verified commit, up to its head, or all of them when no record exists. A run's tests are selected by `check.affected`'s library call with the last green commit as its base, as `clm affected --base <last green commit>` selects them (`graph-test-selection`), written to a file and handed to the check as `CLM_SELECTION` as `[land] check` gets it, so a run covers everything changed since the tests last passed, the lands of an earlier red run included; with no green record there is no selection and every test runs.

**Triggers** (`verify-due`). Read from `[verify] triggers` and `[verify] every` over the unverified lands: `change-done` when one of them adds a `changes/archive/<id>/change.md` (`git diff --diff-filter=A` against its first parent); `queue-drained` when `check.coverage` finds no pending requirement in `origin/main`'s head, checked out into the verify tree (only when `queue-drained` is among the triggers); `work-done` when the caller passed `--event work-done`; `every.lands` when there are at least that many; `every.days` when the oldest of them was committed at least that many days ago. A red last record makes any unverified land a trigger (`red-again`), since a fix may have landed. No unverified land means nothing is due, whatever else holds.

**The run** (`verify-run`). It checks `origin/main`'s head out into `<home>/.clm/verify-tree` as `land` keeps its land tree (`fixed-land-tree`), `<home>` found as `land` finds it, and runs `[verify] check` there through `land`'s shared check runner (`land/shared-check-runner`), so under the clone's check lock, `<home>/.clm/check.lock`, like every other check of the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on the same clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait runs its slow tests in `pipeline` mode or from another clone. Each bisect probe takes the lock afresh, so lands can go between probes. Its output goes to stderr and to `<home>/.clm/verify-logs/<sha>.log`, which a passing run removes and a failing one keeps. `local` and `pipeline` differ only in where the project calls it: `pipeline` is a host pipeline running `clm verify --due` on each push to `main`, with `CLM_WRITER` naming it and the right to push `clm/ledger/*`. A run killed before it records leaves its trigger due. `clm verify --commit <sha>` for a commit other than the head takes the last verified commit before it as its `since` and checks that commit out.

**Import** (`verify-import`). `host`'s `runs` for `[verify] workflow` (`host/list-runs`), each with its head commit, whether it completed and its conclusion. A completed run whose conclusion is `success` or `failure` on an unverified land is a result; the newest such run's commit is the one recorded, covering the unverified lands up to it, `passed` from its conclusion and `run` its URL. Any other conclusion (cancelled, skipped) is no result. With none, `verify pending <head>`, exit 4.

**Culprits and components** (`verify-bisect`). A red result over one unverified land names that land as the culprit. Over several, `local` and `pipeline` runs bisect at once: a binary search of the first-parent lands between the last green commit (exclusive) and the red one, each probe running the same selection in the verify tree, the first red land being the culprit; `import` records `culprit: null` and leaves the search to `clm verify --bisect <sha>` on a developer's machine, which appends a fresh red record for the same commit with the culprit found. The record's components are those whose `paths` hold a file the culprit's first-parent diff changes, or, with no culprit, a file any of the covered lands' diffs change; when no component's `paths` hold such a file (a land that changed only doc-only paths), the root component.

**What it stops.** A red record is a report: `clm intake` lists it for triage (`orchestrator.triage/intake-verify-failures`) and `clm land` warns a branch touching its components. Once the human takes it in as a `kind: bug` item with `verify_failures`, `clm land` holds code lands on the item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands (`land/verify-stop`). A host's run never holds a land by itself (`land-locally`).

## Requirements

### Requirement: verify-due
`clm verify --due` SHALL compute the unverified lands of `origin/main` after fetching it and the ledgers, and SHALL find a run due when there is at least one and either the last `verified` record on `origin/main`'s first-parent chain is red, or one of `[verify] triggers` or `[verify] every` holds: `change-done` when an unverified land adds a file `changes/archive/<id>/change.md`, `queue-drained` when no requirement in `origin/main`'s tree is pending, `work-done` when `--event work-done` was passed and the mode is `local`, `every.lands` when the unverified lands number at least that many, and `every.days` when the oldest of them was committed at least that many days ago. When none holds it SHALL print `verify not-due <why>` and exit 0, appending nothing and running no check. With `--dry-run` it SHALL run and append nothing, print `verify due <trigger>` and exit 0 when a run is due, and print `verify not-due <why>` and exit 3 when not. With no `[verify]` set, every `clm verify` SHALL print `verify off: no [verify] in clm.toml` and exit 0.

#### Scenario: a finished change is due
- GIVEN `[verify]` with `mode = "local"`, `triggers = ["change-done"]`, a `verified` passing record for `origin/main~2`, and two lands since, the second adding `changes/archive/x/change.md`
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due change-done` and exits 0, and the ledger is unchanged

#### Scenario: nothing finished
- GIVEN the same settings, and two lands since the last record, neither adding a file under `changes/archive/`
- WHEN `clm verify --due` runs
- THEN it prints a line starting `verify not-due`, exits 0, runs no check and appends nothing

#### Scenario: work done only when asked
- GIVEN `mode = "local"`, `triggers = ["work-done"]` and one land since the last record
- WHEN `clm verify --due` runs, and then `clm verify --due --event work-done --dry-run`
- THEN the first prints `verify not-due` and the second `verify due work-done`

#### Scenario: backstop by lands
- GIVEN `triggers = []`, `every = { lands = 3 }`, and three lands since the last record
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due every.lands`

#### Scenario: red stays due
- GIVEN no triggers, a red `verified` record for `origin/main~1`, and one land since
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due red-again`

#### Scenario: nothing landed
- GIVEN `triggers = ["work-done"]` and a `verified` record for `origin/main`'s head
- WHEN `clm verify --due --event work-done` runs
- THEN it prints `verify not-due nothing landed since <sha>` and exits 0

#### Scenario: off
- GIVEN a `clm.toml` with no `[verify]`
- WHEN `clm verify --due` runs
- THEN it prints `verify off: no [verify] in clm.toml` and exits 0

#### Needs
- config/verify-setting
- ledger/verified-record

### Requirement: verify-run
In `local` and `pipeline` mode, a due `clm verify --due` SHALL check `origin/main`'s head out into `.clm/verify-tree` in the CLM root of the main worktree and run `[verify] check` there through `land/shared-check-runner`, with `CLM_SELECTION` naming the output of `clm affected --base <last green commit>` when a green record exists and unset otherwise, so under the clone's `.clm/check.lock`, its output copied to stderr and to `.clm/verify-logs/<sha>.log`, removed when the check passes and kept when it fails. It SHALL then append one `verified` record naming the head, the result, the last verified commit as `since`, the unverified lands, the culprit and components (`verify-bisect`), the mode and no `run`, print `verify passed <sha> <n> lands` and exit 0, or print `verify failed <sha> culprit <land|unknown> components <component>...` and the log's path and exit 1. A run that ends before the check exits SHALL append nothing.

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

#### Needs
- config/verify-setting
- ledger/verified-record
- land/shared-check-runner

### Requirement: verify-import
In `import` mode, `clm verify --due` and `clm verify --import` SHALL read `host`'s `runs` for `[verify] workflow` and run no check. The newest completed run whose conclusion is `success` or `failure` on an unverified land SHALL be recorded as a `verified` record for that run's commit, covering the unverified lands up to it, with `passed` from its conclusion, `run` its URL, `culprit` the land when it covers one and null otherwise; a run with any other conclusion, or one not completed, SHALL be no result. With no result it SHALL print `verify pending <head>` and exit 4, appending nothing. `--due` SHALL import only when a trigger holds, as `verify-due` says; `--import` SHALL import whatever result there is. When `host`'s `runs` fails it SHALL exit 1 with its message.

#### Scenario: a passing run imported
- GIVEN `mode = "import"`, `workflow = "slow"`, one land since a passing record, and a fake `gh` reporting a completed run of `slow` on that land with conclusion `success`
- WHEN `clm verify --import` runs
- THEN the ledger holds a `verified` record for that land with `passed: true` and the run's URL, and it exits 0 having run no check

#### Scenario: still running
- GIVEN the same, but the run's status is `in_progress`
- WHEN `clm verify --import` runs
- THEN it prints `verify pending <sha>`, exits 4 and appends nothing

#### Scenario: a cancelled run is no result
- GIVEN a completed run on the land with conclusion `cancelled`
- WHEN `clm verify --import` runs
- THEN it prints `verify pending <sha>` and appends nothing

#### Scenario: nothing to import elsewhere
- GIVEN `mode = "local"`
- WHEN `clm verify --import` runs
- THEN it prints `verify nothing to import`, exits 0 and calls no `gh`

#### Needs
- config/verify-setting
- ledger/verified-record
- host/list-runs

### Requirement: verify-commit
`clm verify --commit <sha>` SHALL refuse with exit 1 a commit that isn't on `origin/main`'s first-parent chain. For one that is, it SHALL answer from the ledger when a `verified` record names it, printing `verify passed` or `verify failed` and exiting 0 or 1, and otherwise verify it as `verify-run` or `verify-import` verifies `origin/main`'s head, triggers aside, with the commit checked out and the last verified commit before it as `since`, appending the record.

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

#### Needs
- config/verify-setting
- host/list-runs
- ledger/verified-record

### Requirement: verify-bisect
When `verify-run` finds a red result over more than one unverified land, it SHALL find the culprit by binary search over the first-parent lands after the last green commit up to the red one, running `[verify] check` with the same selection on each probe in the verify tree, the culprit being the first land on which it fails, and SHALL record it and the components whose `paths` hold a file the culprit's first-parent diff changes, or the root component when none does. Over one unverified land, that land is the culprit. In `import` mode the record SHALL have `culprit: null` and the components of every covered land. `clm verify --bisect <sha>`, for a commit a red record names, SHALL run the same search on the machine that runs it and append a red record for that commit naming the culprit and its components.

#### Scenario: the breaking land found
- GIVEN a passing record for `main`'s commit `g`, three lands after it, `l1` changing `crates/a/x.rs`, `l2` changing `crates/b/y.rs` and `l3` changing `crates/a/z.rs`, components `a` and `b` owning `crates/a/**` and `crates/b/**`, and a `[verify] check` that fails on any tree holding `l2`'s change
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify failed <l3> culprit <l2> components b`, and the record names `l2` and `[b]`

#### Scenario: bisect an imported failure
- GIVEN `mode = "import"`, a red record for `l3` with `culprit: null`, and the same check and lands
- WHEN `clm verify --bisect l3` runs
- THEN a red record for `l3` naming `l2` and `[b]` is appended

#### Needs
- config/verify-setting
- ledger/verified-record
- land/shared-check-runner
