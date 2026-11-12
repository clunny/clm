---
title: Verify
status: active
owners: ["@clunny"]
depends_on: [config, ledger, host, vcs, model, check.coverage, check.affected, land]
paths: ["crates/clm-verify/**"]
serves: [capability/slow-tests-after-land]
---

## Contract
Running a project's slow tests after the land, at the moments the project chooses, and recording the result in the ledger (`verify-after-land`). Every `clm verify` takes its `[verify]` and `[affected]` settings, and the `[land] target_cap_gb` that caps its build place, from the `clm.toml` committed at `origin/main`, which it fetches first, never from the checkout it runs in, so a checkout behind `main` judges `main` by `main`'s settings (`settings-from-main`). A project whose `origin/main` sets no `[verify]` (`config`) keeps its slow tests in `[land] check`, and every `clm verify` then prints `verify off: no [verify] in origin/main:clm.toml` and exits 0. Crate `clm-verify` offers, dispatched by `crates/clm`:
- `clm verify --due [--event work-done] [--dry-run]`: whether a trigger has fired since the last verified commit of `origin/main` (`verify-due`), and when one has, the verification of `origin/main`'s head: in `local` and `pipeline` mode by running `[verify] check` (`verify-run`), in `import` mode by reading the pipeline's result (`verify-import`). With `--dry-run` it only says whether a run is due.
- `clm verify --commit <sha>`: the verification of one commit of `main`, for a release or deploy step (`verify-commit`).
- `clm verify --import`: in `import` mode, the record of every completed pipeline run on `main` newer than the last record, run nothing; in the other modes it prints `verify nothing to import` and exits 0.
- `clm verify --bisect <sha>`: the land that broke a red commit, found by running `[verify] check` on the lands it covers (`verify-bisect`).

Every outcome is one line on stdout starting `verify `: `verify not-due <why>`, `verify due <trigger>`, `verify passed <sha> <n> lands`, `verify failed <sha> culprit <land|unknown> components <component>...`, `verify pending <sha>` and `verify nothing to import`, `<sha>` abbreviated as `git rev-parse --short` abbreviates it. It exits 0 when it passed, nothing was due or there was nothing to import; 1 when the verification failed, or with the reason when it can't run (a `<sha>` not on `origin/main`'s first parents, a `[verify]` it can't use); 2 when git fails; 3 when, with `--dry-run`, nothing is due; and 4 when the result is pending (`import`). Each verification it decides appends one `verified` record (`ledger`); nothing else writes one.

As a library call it offers `kept_log(root, commit)`: the path of the log a failed run of `commit` kept in this clone, or none (`kept-log`), so `orchestrator.triage`'s `clm intake` can point a session at a red result's output without knowing where verify keeps it (`path-ownership`).

## Design
**Settings** (`settings-from-main`). `clm verify` verifies `origin/main`, so it reads how to verify it from `origin/main`: the commit it checks out is the one whose settings it follows. It first loads the root's own settings for `host` alone, which names the host adapter, fetches `origin` (every host has one: `none-keeps-its-own-remote`), and then loads the settings again with `config`'s `load_committed`, the repository layer being the text of `origin/main:<prefix>clm.toml` (none when `origin/main` or the file is missing) and its source `origin/main:clm.toml`. `[verify]`, `[affected] all` and `[land] target_cap_gb`, the cap on verify's build place (`verify-build-cap`), come from those; the clone's `clm.local.toml` still sets either for this clone, as everywhere (`local-settings`), since it belongs to the clone and is never behind. Whether `[verify]` is set is decided only after the fetch, so a project with no `[verify]` pays one fetch per call; a fetch that fails is exit 2 as for any other call. The sessions that run `clm verify --due --event work-done` at their end (`plugin/skills-run-verify`) run it in their own checkout, which they leave as it is and which may be far behind `main`: reading the checkout's file there skipped every run with `verify off` once `[verify]` had landed after the checkout was made. `host` stays the root's, as `land` keeps it (`land-policy-from-base`): it is needed before there is anything fetched to read, and names the tooling, not the policy. This is the decision `verify-settings-from-main`.

**Since when.** The *last verified commit* is the newest commit on `origin/main`'s first-parent chain that a `verified` record names, whatever its result; the *last green commit* the newest that one with `passed: true` names. The *unverified lands* are the first-parent commits of `origin/main` after the last verified commit, up to its head, or all of them when no record exists. A run's tests are selected by `check.affected`'s library call with the last green commit as its base, as `clm affected --base <last green commit>` selects them (`graph-test-selection`), written to a file and handed to the check as `CLM_SELECTION` as `[land] check` gets it, so a run covers everything changed since the tests last passed, the lands of an earlier red run included; with no green record there is no selection and every test runs.

**Triggers** (`verify-due`). Read from `[verify] triggers` and `[verify] every` over the unverified lands: `change-done` when one of them adds a `changes/archive/<id>/change.md` (`git diff --diff-filter=A` against its first parent); `queue-drained` when `check.coverage` finds no pending requirement in `origin/main`'s head, checked out into the verify tree (only when `queue-drained` is among the triggers); `work-done` when the caller passed `--event work-done`; `every.lands` when there are at least that many; `every.days` when the oldest of them was committed at least that many days ago. A red last record makes any unverified land a trigger (`red-again`), since a fix may have landed. No unverified land means nothing is due, whatever else holds.

**The run** (`verify-run`). It checks `origin/main`'s head out into `<home>/.clm/verify-tree` as `land` keeps its land tree (`fixed-land-tree`), `<home>` found as `land` finds it, and runs `[verify] check` there through `land`'s shared check runner (`land/shared-check-runner`), so under the clone's check lock, `<home>/.clm/check.lock`, like every other check of the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on the same clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait runs its slow tests in `pipeline` mode or from another clone. Each bisect probe takes the lock afresh, so lands can go between probes. **Build place** (`verify-build-dir`, `verify-build-cap`). Every run of `[verify] check`, a bisect probe's included, gets `CLM_LAND_DIR` set to `<home>/.clm/verify-build`, as a land's check gets `.clm/land` (`land/land-dir-env`): a place that lasts from run to run outside the verify tree, so a check script that points its toolchain there for lands (`init/starter-check-land-dir`) does so for verify too, with incremental compilation off. Build output kept inside the verify tree, which keeps what git ignores from run to run, grew without bound: 27 GB on one clone, 15 GB of it incremental data no run between whole commits reuses. Verify sets no toolchain's variables itself. After each run of the check, still holding the verify tree's lock (`verify.lock`), so no other verify run is building there, it applies `land`'s cap to that place through `land/shared-target-cap`, named `.clm/verify-build` and capped at `[land] target_cap_gb` from the settings it follows (`settings-from-main`), printing its line on stderr as the check's other notes go; a warning from the cap never changes the run's record or exit status. Cargo prunes no unit no run builds again, so the cap trades one cold run for a bounded place, as the land's does. Its output goes to stderr and to `<home>/.clm/verify-logs/<sha>.log`, which a passing run removes and a failing one keeps. `local` and `pipeline` differ only in where the project calls it: `pipeline` is a host pipeline running `clm verify --due` on each push to `main`, with `CLM_WRITER` naming it and the right to push `clm/ledger/*`. A run killed before it records leaves its trigger due. **Output** (`verify-run`). Once the check has exited, nothing `clm verify` writes can lose its result. Every write to stderr before the record (the verify tree's waiting line, a failed selection's `clm affected failed` note, the check runner's waiting line and log warning, and each bisect probe's output and lines) ignores a failure, as the runner's copy of the check's output already does, and nothing goes to stdout before it: the outcome line and the notes are printed only after the record is appended, so a stdout whose reader has gone (`clm/closed-stdout`), or a stream on a full disk, ends the command with the verdict already in the ledger. A failed write before the check exits ends nothing either, since ending then would leave the check running unwatched in the verify tree. The check's output is still copied to stderr, so a person watching a run sees it progress; the log holds it whole. `clm verify --commit <sha>` for a commit other than the head takes the last verified commit before it as its `since` and checks that commit out.

**Import** (`verify-import`). `host`'s `runs` for `[verify] workflow` (`host/list-runs`), each with its head commit, whether it completed and its conclusion. A completed run whose conclusion is `success` or `failure` on an unverified land is a result; the newest such run's commit is the one recorded, covering the unverified lands up to it, `passed` from its conclusion and `run` its URL. Any other conclusion (cancelled, skipped) is no result. With none, `verify pending <head>`, exit 4.

**Culprits and components** (`verify-bisect`). A red result over one unverified land names that land as the culprit. Over several, `local` and `pipeline` runs bisect at once: a search of the first-parent lands between the last green commit (exclusive) and the red one, each probe running the same selection in the verify tree, the first red land being the culprit; `import` records `culprit: null` and leaves the search to `clm verify --bisect <sha>` on a developer's machine, which appends a fresh red record for the same commit with what the search found. **Untestable lands.** A probe whose `[verify] check` exits 125, 126 or 127 could not test its land: 126 and 127 are `sh`'s own codes for a command it found but could not run and one it could not find, as on a commit older than the project's slow-test script, and 125 is the code `git bisect run` already gives a script for "this commit can't be tested", so a project's script can say so without CLM inventing a convention (`reuse-first`). Such a land is set aside, neither green nor red. The search keeps a range of lands `lo..hi`, `hi` the red one (at first the red commit itself) and every land before `lo` in the range known green; it probes the land in `lo..hi` (`hi` excluded) not set aside that is nearest `lo + (hi - lo) / 2` (rounded down), the later of two equally near, moving `lo` past a green probe and `hi` to a red one, until no such land is left. When `lo` is `hi`, that land is the culprit. Otherwise every land from `lo` to `hi` but the last was set aside and any of them may hold the break, so the culprit is unknown, and stderr says, after the outcome line, `culprit unknown: [verify] check could not run on <n> lands, <short lo> to <short hi-1>, before the first red one, <short hi>`. The search assumes, as `git bisect` does, that a land after a red one stays red; to conclude it is unknown it must probe every land it set aside, each a checkout and a check that ends at once. The record's components are those whose `paths` hold a file the culprit's first-parent diff changes, or, with no culprit, a file any of the lands that may hold it change: those from `lo` to `hi` when the search left it unknown, and in `import` mode, which searches nothing, the covered lands. A land with no first parent (the repository's first commit) diffs against the empty tree. When no component's `paths` hold such a file (a land that changed only doc-only paths), the root component.

**What it stops.** A red record is a report: `clm intake` lists it for triage (`orchestrator.triage/intake-verify-failures`) and `clm land` warns a branch touching its components. Once the human takes it in as a `kind: bug` item with `verify_failures`, `clm land` holds code lands on the item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands, letting through meanwhile a branch that resolves another item holding the same component (`land/verify-stop`). A host's run never holds a land by itself (`land-locally`).

## Requirements

### Requirement: verify-due
`clm verify --due` SHALL compute the unverified lands of `origin/main` after fetching it and the ledgers, and SHALL find a run due when there is at least one and either the last `verified` record on `origin/main`'s first-parent chain is red, or one of `[verify] triggers` or `[verify] every` holds: `change-done` when an unverified land adds a file `changes/archive/<id>/change.md`, `queue-drained` when no requirement in `origin/main`'s tree is pending, `work-done` when `--event work-done` was passed and the mode is `local`, `every.lands` when the unverified lands number at least that many, and `every.days` when the oldest of them was committed at least that many days ago, the settings being those `settings-from-main` reads. When none holds it SHALL print `verify not-due <why>` and exit 0, appending nothing and running no check. With `--dry-run` it SHALL run and append nothing, print `verify due <trigger>` and exit 0 when a run is due, and print `verify not-due <why>` and exit 3 when not. With no `[verify]` set there, every `clm verify` SHALL print `verify off: no [verify] in origin/main:clm.toml` and exit 0.

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
- GIVEN an `origin/main` whose `clm.toml` has no `[verify]`
- WHEN `clm verify --due` runs
- THEN it prints `verify off: no [verify] in origin/main:clm.toml` and exits 0

#### Needs
- config/verify-setting
- config/committed-settings
- ledger/verified-record

### Requirement: verify-run
In `local` and `pipeline` mode, a due `clm verify --due` SHALL check `origin/main`'s head out into `.clm/verify-tree` in the CLM root of the main worktree and run `[verify] check` there through `land/shared-check-runner`, with `CLM_SELECTION` naming the output of `clm affected --base <last green commit>` when a green record exists and unset otherwise, so under the clone's `.clm/check.lock`, its output copied to stderr and to `.clm/verify-logs/<sha>.log`, removed when the check passes and kept when it fails. It SHALL then append one `verified` record naming the head, the result, the last verified commit as `since`, the unverified lands, the culprit and components (`verify-bisect`), the mode and no `run`, print `verify passed <sha> <n> lands` and exit 0, or print `verify failed <sha> culprit <land|unknown> components <component>...` and the log's path and exit 1. When the check fails, it SHALL then print on stderr, after its other notes and just before the log's path, what `land/check-log`'s refusal holds of the run of the check on the commit it verifies (the head, or the commit `--commit` names), not of a bisect probe: the line ``last 40 lines of `<check>`:``, the last 40 lines of that run's output, and, when a step was open as its output ended (`check-step-markers`), the line ``the step open when it ended is `<name>`, lines <first> to <last> of its output``, counted from 1 in the whole output. The step and the lines SHALL be those the shared check runner returns, and a check that marks no step SHALL get no step line. A result answered from the ledger, a `--bisect` search and an `import` result run no check there and SHALL print no such report. A run that ends before the check exits SHALL append nothing. A write to stderr that fails SHALL neither end the run nor change its result or exit status before the record is appended, and nothing SHALL be printed to stdout before it, so once the check has exited its record is appended whatever becomes of `clm verify`'s stdout and stderr; what the command prints after the record MAY still end it, as `clm/closed-stdout` says.

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

### Requirement: verify-bisect
When `verify-run` finds a red result over more than one unverified land, it SHALL find the culprit by binary search over the first-parent lands after the last green commit up to the red one, running `[verify] check` with the same selection on each probe in the verify tree, the culprit being the first land on which it fails. A probe whose check exits 125, 126 or 127 SHALL count as untestable, neither green nor red: the search SHALL set that land aside and probe another of the range left, and SHALL end when the range left holds no land it has not probed. When it ends with untestable lands between the last green probe (or the range's start) and the first red land, the culprit SHALL be unknown: the record SHALL have `culprit: null` and the components whose `paths` hold a file any of those lands or the first red land changes, the outcome SHALL read `culprit unknown`, and stderr SHALL say, after it, `culprit unknown: [verify] check could not run on <n> lands, <first> to <last>, before the first red one, <red>`, each abbreviated as `git rev-parse --short` abbreviates it and `lands` whatever `<n>`; only the run that searched prints it, not `--commit` answering from the record. Otherwise it SHALL record the culprit and the components whose `paths` hold a file the culprit's first-parent diff changes. A land with no first parent SHALL be diffed against the empty tree, and when no component's `paths` hold a changed file the components SHALL be the root component. Over one unverified land, that land is the culprit. In `import` mode the record SHALL have `culprit: null` and the components of every covered land. `clm verify --bisect <sha>`, for a commit a red record names, SHALL run the same search on the machine that runs it and append a red record for that commit naming the culprit and its components, or `culprit: null` and the components of the lands that may hold it when the search leaves it unknown.

#### Scenario: the breaking land found
- GIVEN a passing record for `main`'s commit `g`, three lands after it, `l1` changing `crates/a/x.rs`, `l2` changing `crates/b/y.rs` and `l3` changing `crates/a/z.rs`, components `a` and `b` owning `crates/a/**` and `crates/b/**`, and a `[verify] check` that fails on any tree holding `l2`'s change
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify failed <l3> culprit <l2> components b`, and the record names `l2` and `[b]`

#### Scenario: bisect an imported failure
- GIVEN `mode = "import"`, a red record for `l3` with `culprit: null`, and the same check and lands
- WHEN `clm verify --bisect l3` runs
- THEN a red record for `l3` naming `l2` and `[b]` is appended

#### Scenario: an untestable land is skipped
- GIVEN the breaking-land fixture with two more lands, `l4` and `l5`, each changing a file under `crates/a/`, and a `[verify] check` that exits 127 on `l3`'s tree and otherwise fails on any tree holding `l2`'s change
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify failed <l5> culprit <l2> components b`, and stderr holds no `culprit unknown` line

#### Scenario: no land before the red one could be tested
- GIVEN no `verified` record, lands `l1` changing `crates/a/x.rs`, `l2` changing `crates/b/y.rs` and `l3` changing `crates/a/z.rs` on a chain whose first commit `r` adds `README.md`, and a `[verify] check` that exits 127 on every tree but `l3`'s, where it exits 1
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify failed <l3> culprit unknown components a b`, stderr holds `culprit unknown: [verify] check could not run on 3 lands, <r> to <l2>, before the first red one, <l3>`, and the record has `culprit: null` and `[a, b]`

#### Scenario: a script says it can't test
- GIVEN the breaking-land fixture and a `[verify] check` that exits 125 on `l2`'s tree, 0 on `l1`'s and 1 on `l3`'s
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify failed <l3> culprit unknown components a b`, and stderr holds `culprit unknown: [verify] check could not run on 1 lands, <l2> to <l2>, before the first red one, <l3>`

#### Needs
- config/verify-setting
- ledger/verified-record
- land/shared-check-runner

### Requirement: culprit-recheck
When `verify-bisect` names a culprit land whose `landed` record says it re-checked with `none` or `overlap` (`ledger/landed-recheck`), `clm verify` SHALL print on stderr, after its outcome line, `culprit <short sha> re-checked <how> at land; its branch's check and main's were composed from <head key> and <base key>`, `<how>` being `nothing` for `none` and `the overlap, <n> files` (or `the overlap, every test`) for `overlap`, and the keys those of the composed `checked` record for its code key, abbreviated to 12 characters, or `unknown` for both when the fold holds no such record. It SHALL read the culprit's `landed` record and its `checked` records from the fold's lands and checks. It SHALL say nothing of the failure's cause: a missing fact in the graph, the code and a flaky test all fail this way. Its stdout and the `verified` record SHALL be unchanged.

#### Scenario: a culprit that ran no check
- GIVEN `verify-bisect`'s breaking-land fixture, with `l2`'s `landed` record holding `recheck: none` and a composed `checked` record for its code key with basis `h` and `b`
- WHEN `clm verify --due` runs with a trigger holding
- THEN stdout is as `verify-bisect` says, and stderr holds `culprit <l2> re-checked nothing at land` and both keys

#### Scenario: a culprit that ran its own selection
- GIVEN the same with `recheck: selected`
- WHEN `clm verify --due` runs
- THEN stderr holds no `culprit` line

#### Needs
- ledger/landed-recheck
- ledger/composed-check

### Requirement: settings-from-main
Every `clm verify` SHALL take `host` from the settings of the CLM root it runs in, and, after fetching `origin`, on every host, every other setting it uses (`[verify]`, `[affected]`) but `[env]`, which `verify-env` takes from the commit it checks, from `config`'s `load_committed` with the text of the CLM root's `clm.toml` committed at `origin/main`, `None` when `origin/main` or that file doesn't exist, and `source` `origin/main:clm.toml`. It SHALL never take `[verify]` or `[affected]` from the working tree's `clm.toml`, and SHALL decide whether `[verify]` is set only after the fetch. When those settings set no `[verify]` it SHALL print `verify off: no [verify] in origin/main:clm.toml` and exit 0, running and appending nothing; when they fail to load it SHALL exit 1 with `config`'s message, which names `origin/main:clm.toml`. A fetch that fails SHALL exit 2, as git failures do.

#### Scenario: a checkout behind main
- GIVEN a clone whose checkout's `clm.toml` sets no `[verify]`, and an `origin/main` ahead of it whose `clm.toml` sets `[verify]` with `mode = "local"` and `triggers = ["work-done"]`, with one land since a passing `verified` record
- WHEN `clm verify --due --event work-done --dry-run` runs in the checkout
- THEN it prints `verify due work-done` and exits 0

#### Scenario: turned off on main
- GIVEN a checkout whose `clm.toml` sets `[verify]` with a `check` that writes a file, and an `origin/main` whose `clm.toml` sets none
- WHEN `clm verify --due --event work-done` runs in the checkout
- THEN it prints `verify off: no [verify] in origin/main:clm.toml`, exits 0, and the file isn't written

#### Scenario: an invalid setting on main
- GIVEN an `origin/main` whose `clm.toml` sets `[verify]` with no `check`
- WHEN `clm verify --due` runs
- THEN it exits 1 naming `origin/main:clm.toml` and `verify`, and appends nothing

#### Needs
- config/committed-settings
- config/verify-setting

### Requirement: verify-build-dir
Every run of `[verify] check` by `clm verify`, in each of `--due`, `--commit` and `--bisect` and each bisect probe, SHALL set `CLM_LAND_DIR` in the command's environment to the `.clm/verify-build` directory of the main worktree's CLM root, replacing any value the caller's environment holds, and SHALL set neither `CARGO_TARGET_DIR` nor `CARGO_INCREMENTAL`, leaving both as the caller's environment has them.

#### Scenario: a run's place
- GIVEN `mode = "local"`, one land since a passing record, a `[verify] check` that writes `$CLM_LAND_DIR` and `$CARGO_TARGET_DIR` to a file outside the repository and exits 0, and `CLM_LAND_DIR=/elsewhere` and `CARGO_TARGET_DIR=/mine` in the caller's environment
- WHEN `clm verify --due` runs from a linked worktree with a trigger holding
- THEN the file holds the path of the main worktree's `.clm/verify-build` and `/mine`

#### Scenario: a bisect probe's place
- GIVEN two lands since a passing record and a `[verify] check` that appends `$CLM_LAND_DIR` to a file outside the repository and fails on any tree holding the second land's change
- WHEN `clm verify --due` runs with a trigger holding
- THEN every line of the file is the path of the main worktree's `.clm/verify-build`, and it has a line for each probe

### Requirement: verify-build-cap
After each run of `[verify] check` by `clm verify`, a bisect probe's included, and while it still holds the verify tree's lock, `clm verify` SHALL apply `land/shared-target-cap` to the main worktree's `.clm/verify-build`, named `.clm/verify-build`, with the cap `[land] target_cap_gb` of the settings it follows (15 when unset), and print the line the call returns on stderr. Whether the run passed or failed SHALL not decide whether the cap is applied, and nothing the cap returns SHALL change the run's record or exit status.

#### Scenario: over the cap
- GIVEN `mode = "local"`, `[land] target_cap_gb = 1`, one land since a passing record, and a `[verify] check` that creates a sparse file of 2,000,000,000 bytes in `CLM_LAND_DIR` and exits 0
- WHEN `clm verify --due` runs with a trigger holding
- THEN it prints `verify passed`, stderr holds `cleared .clm/verify-build (2 GB, cap 1 GB)`, and the main worktree's `.clm/verify-build` doesn't exist

#### Scenario: under the cap
- GIVEN the same, with a check that creates a file of 10 bytes there and the cap left at its default
- WHEN `clm verify --due` runs with a trigger holding
- THEN stderr holds no `cleared` line and the file is still there

#### Scenario: a red run is capped too
- GIVEN the over-the-cap settings, with a check that creates its file and exits 1
- WHEN `clm verify --due` runs with a trigger holding
- THEN it exits 1, the ledger holds a `verified` record with `passed: false`, and stderr holds `cleared .clm/verify-build (2 GB, cap 1 GB)`

#### Needs
- land/shared-target-cap

### Requirement: verify-env
Every run of `[verify] check`, a bisect probe's included, SHALL set in the command's environment each variable of the `env` (`config/env-setting`) of the commit it checks out (`origin/main`'s head, the commit `--commit` names, or the probe's land): the one `config`'s `committed_env` gives for the CLM root's `clm.toml` at that commit, with the clone's `clm.local.toml`, never the working tree's `clm.toml`, each replacing any value the caller's environment holds for that name. When there is a last green commit and the `env` of the commit it verifies differs from the one `committed_env` gives for the green commit's `clm.toml` with the same `clm.local.toml`, every run of the check for that verification, its probes included, SHALL have `CLM_SELECTION` unset, since a variable can change any test's result. `CLM_LAND_DIR` (`verify-build-dir`) SHALL be as it is without it.

#### Scenario: main's table and the clone's
- GIVEN a `[verify] check` that writes `$A` and `$B` to a file outside the repository, a checkout whose `clm.toml` sets `[env]` with `A = "checkout"`, an `origin/main` whose `clm.toml` sets `[verify]` with that check and `[env]` with `A = "main"` and `B = "main"`, a `clm.local.toml` with `[env]` setting `B = "local"`, `A=caller` in the caller's environment, and a run due
- WHEN `clm verify --due` runs in the checkout
- THEN the file holds `main` for `A` and `local` for `B`

#### Scenario: a probe takes its land's variables
- GIVEN a passing record, then a land setting `[env]` `A = "1"`, then a land setting `A = "2"`, and a `[verify] check` that fails when `$A` is `2`
- WHEN `clm verify --due` runs and bisects
- THEN the probe of the first land ran with `A` set to `1` and passed, and the record names the second land as the culprit

#### Scenario: a changed variable runs every test
- GIVEN a passing record, then one land that changes only `clm.toml`'s `[env]`, and a `[verify] check` that writes whether `CLM_SELECTION` is set
- WHEN `clm verify --due` runs
- THEN the check ran with `CLM_SELECTION` unset

#### Needs
- config/env-setting

### Requirement: kept-log
`clm-verify` SHALL offer `kept_log(root, commit)`, `root` a CLM root in any worktree of a clone and `commit` a full commit ID, returning the absolute path of the file in `.clm/verify-logs/` in the CLM root of the clone's main worktree, the directory `verify-run` keeps a failed run's log in, whose name is `<name>.log` with `<name>` a prefix of `commit` at least 7 characters long, the longest such name when several match, and none when no file matches or the directory is missing. It SHALL read only that directory, running no git command other than the one that finds the main worktree, and SHALL neither create nor remove a file. Matching by prefix rather than by the abbreviation `git rev-parse --short` gives now keeps a log found after the repository's abbreviations have grown longer than when the run named it.

#### Scenario: a red run's log
- GIVEN a clone with a linked worktree, `mode = "local"`, one land since a passing record and a `[verify] check` that exits 1, and `clm verify --due` run in the main worktree with a trigger holding
- WHEN `kept_log` is called with the linked worktree's CLM root and the head's full commit ID
- THEN it returns the path of the log `clm verify` named, which exists

#### Scenario: nothing kept
- GIVEN the same clone, and a `[verify] check` that exits 0
- WHEN `clm verify --due` runs and `kept_log` is called with the head's full commit ID
- THEN it returns none

#### Scenario: a longer name
- GIVEN `.clm/verify-logs/` holding `<first 7 of commit>.log` and `<first 9 of commit>.log`
- WHEN `kept_log` is called with `commit`
- THEN it returns the path of `<first 9 of commit>.log`
