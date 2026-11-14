---
component: verify
base: { contract: "85bb62fe0d2e29f8", design: "0cb67f6022159217" }
---

## MODIFIED Contract

Running a project's slow tests after the land, at the moments the project chooses, and recording the result in the ledger (`verify-after-land`). Every `clm verify` takes its `[verify]` and `[affected]` settings, and the `[land] target_cap_gb` that caps its build place, from the `clm.toml` committed at `origin/main`, which it fetches first, never from the checkout it runs in, so a checkout behind `main` judges `main` by `main`'s settings (`settings-from-main`). A project whose `origin/main` sets no `[verify]` (`config`) keeps its slow tests in `[land] check`, and every `clm verify` then prints `verify off: no [verify] in origin/main:clm.toml` and exits 0. Crate `clm-verify` offers, dispatched by `crates/clm`:
- `clm verify --due [--event work-done] [--dry-run]`: whether a trigger has fired since the last verified commit of `origin/main` (`verify-due`), and when one has, the verification of `origin/main`'s head: in `local` and `pipeline` mode by running `[verify] check` (`verify-run`), in `import` mode by reading the pipeline's result (`verify-import`). With `--dry-run` it only says whether a run is due.
- `clm verify --commit <sha>`: the verification of one commit of `main`, for a release or deploy step (`verify-commit`).
- `clm verify --import`: in `import` mode, the record of every completed pipeline run on `main` newer than the last record, run nothing; in the other modes it prints `verify nothing to import` and exits 0.
- `clm verify --bisect <sha>`: the land that broke a red commit, found by running `[verify] check` on the lands it covers (`verify-bisect`).

Every outcome is one line on stdout starting `verify `: `verify not-due <why>`, `verify due <trigger>`, `verify passed <sha> <n> lands`, `verify failed <sha> culprit <land|unknown> components <component>...`, `verify pending <sha>` and `verify nothing to import`, `<sha>` abbreviated as `git rev-parse --short` abbreviates it. It exits 0 when it passed, nothing was due or there was nothing to import; 1 when the verification failed, or with the reason when it can't run (a `<sha>` not on `origin/main`'s first parents, a `[verify]` it can't use); 2 when git fails; 3 when, with `--dry-run`, nothing is due; and 4 when the result is pending (`import`). Each verification it decides appends one `verified` record (`ledger`); nothing else writes one.

When the settings it follows set `[verify] mutate`, a run whose check passed goes on to the mutation pass (`mutation-pass`, `mutation-after-land`): it asks the project's command whether the tests that cite each requirement built since the last pass notice a change to that requirement's component, appends one `mutated` record, and lands a `bug` item naming each requirement whose tests noticed none as `unproven` (`files-unproven`), which reads as not built until a build resolves the item (`check.coverage/coverage-report`). The pass changes neither the outcome line nor the exit status, and holds no lock a land waits for.

As a library call it offers `kept_log(root, commit)`: the path of the log a failed run of `commit` kept in this clone, or none (`kept-log`), so `orchestrator.triage`'s `clm intake` can point a session at a red result's output without knowing where verify keeps it (`path-ownership`).

## MODIFIED Design

**Settings** (`settings-from-main`). `clm verify` verifies `origin/main`, so it reads how to verify it from `origin/main`: the commit it checks out is the one whose settings it follows. It first loads the root's own settings for `host` alone, which names the host adapter, fetches `origin` (every host has one: `none-keeps-its-own-remote`), and then loads the settings again with `config`'s `load_committed`, the repository layer being the text of `origin/main:<prefix>clm.toml` (none when `origin/main` or the file is missing) and its source `origin/main:clm.toml`. `[verify]`, `[affected] all` and `[land] target_cap_gb`, the cap on verify's build place (`verify-build-cap`), come from those; the clone's `clm.local.toml` still sets either for this clone, as everywhere (`local-settings`), since it belongs to the clone and is never behind. Whether `[verify]` is set is decided only after the fetch, so a project with no `[verify]` pays one fetch per call; a fetch that fails is exit 2 as for any other call. The sessions that run `clm verify --due --event work-done` at their end run it in their own checkout, which they leave as it is and which may be far behind `main`: reading the checkout's file there skipped every run with `verify off` once `[verify]` had landed after the checkout was made. `host` stays the root's, as `land` keeps it (`land-policy-from-base`): it is needed before there is anything fetched to read, and names the tooling, not the policy. This is the decision `verify-settings-from-main`.

**Since when.** The *last verified commit* is the newest commit on `origin/main`'s first-parent chain that a `verified` record names, whatever its result; the *last green commit* the newest that one with `passed: true` names. The *unverified lands* are the first-parent commits of `origin/main` after the last verified commit, up to its head, or all of them when no record exists. A run's tests are selected by `check.affected`'s library call with the last green commit as its base, as `clm affected --base <last green commit>` selects them (`graph-test-selection`), written to a file and handed to the check as `CLM_SELECTION` as `[land] check` gets it, so a run covers everything changed since the tests last passed, the lands of an earlier red run included; with no green record there is no selection and every test runs.

**Triggers** (`verify-due`). Read from `[verify] triggers` and `[verify] every` over the unverified lands: `change-done` when one of them adds a `changes/archive/<id>/change.md` (`git diff --diff-filter=A` against its first parent); `queue-drained` when `check.coverage` finds no pending requirement in `origin/main`'s head, checked out into the verify tree (only when `queue-drained` is among the triggers); `work-done` when the caller passed `--event work-done`; `every.lands` when there are at least that many; `every.days` when the oldest of them was committed at least that many days ago. A red last record makes any unverified land a trigger (`red-again`), since a fix may have landed. No unverified land means nothing is due, whatever else holds.

**The run** (`verify-run`). It checks `origin/main`'s head out into `<home>/.clm/verify-tree` as `land` keeps its land tree (`fixed-land-tree`), `<home>` found as `land` finds it, and runs `[verify] check` there through `land`'s shared check runner (`land/shared-check-runner`), so under the clone's check lock, `<home>/.clm/check.lock`, like every other check of the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on the same clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait runs its slow tests in `pipeline` mode or from another clone. Each bisect probe takes the lock afresh, so lands can go between probes. **Build place** (`verify-build-dir`, `verify-build-cap`). Every run of `[verify] check`, a bisect probe's included, gets `CLM_LAND_DIR` set to `<home>/.clm/verify-build`, as a land's check gets `.clm/land` (`land/land-dir-env`): a place that lasts from run to run outside the verify tree, so a check script that points its toolchain there for lands (`init/starter-check-land-dir`) does so for verify too, with incremental compilation off. Build output kept inside the verify tree, which keeps what git ignores from run to run, grew without bound: 27 GB on one clone, 15 GB of it incremental data no run between whole commits reuses. Verify sets no toolchain's variables itself. After each run of the check, still holding the verify tree's lock (`verify.lock`), so no other verify run is building there, it applies `land`'s cap to that place through `land/shared-target-cap`, named `.clm/verify-build` and capped at `[land] target_cap_gb` from the settings it follows (`settings-from-main`), printing its line on stderr as the check's other notes go; a warning from the cap never changes the run's record or exit status. Cargo prunes no unit no run builds again, so the cap trades one cold run for a bounded place, as the land's does. Its output goes to stderr and to `<home>/.clm/verify-logs/<sha>.log`, which a passing run removes and a failing one keeps. `local` and `pipeline` differ only in where the project calls it: `pipeline` is a host pipeline running `clm verify --due` on each push to `main`, with `CLM_WRITER` naming it and the right to push `clm/ledger/*`. A run killed before it records leaves its trigger due. **Output** (`verify-run`). Once the check has exited, nothing `clm verify` writes can lose its result. Every write to stderr before the record (the verify tree's waiting line, a failed selection's `clm affected failed` note, the check runner's waiting line and log warning, and each bisect probe's output and lines) ignores a failure, as the runner's copy of the check's output already does, and nothing goes to stdout before it: the outcome line and the notes are printed only after the record is appended, so a stdout whose reader has gone (`clm/closed-stdout`), or a stream on a full disk, ends the command with the verdict already in the ledger. A failed write before the check exits ends nothing either, since ending then would leave the check running unwatched in the verify tree. The check's output is still copied to stderr, so a person watching a run sees it progress; the log holds it whole. `clm verify --commit <sha>` for a commit other than the head takes the last verified commit before it as its `since` and checks that commit out.

**Import** (`verify-import`). `host`'s `runs` for `[verify] workflow` (`host/list-runs`), each with its head commit, whether it completed and its conclusion. A completed run whose conclusion is `success` or `failure` on an unverified land is a result; the newest such run's commit is the one recorded, covering the unverified lands up to it, `passed` from its conclusion and `run` its URL. Any other conclusion (cancelled, skipped) is no result. With none, `verify pending <head>`, exit 4.

**Culprits and components** (`verify-bisect`). A red result over one unverified land names that land as the culprit. Over several, `local` and `pipeline` runs bisect at once: a search of the first-parent lands between the last green commit (exclusive) and the red one, each probe running the same selection in the verify tree, the first red land being the culprit; `import` records `culprit: null` and leaves the search to `clm verify --bisect <sha>` on a developer's machine, which appends a fresh red record for the same commit with what the search found. **Untestable lands.** A probe whose `[verify] check` exits 125, 126 or 127 could not test its land: 126 and 127 are `sh`'s own codes for a command it found but could not run and one it could not find, as on a commit older than the project's slow-test script, and 125 is the code `git bisect run` already gives a script for "this commit can't be tested", so a project's script can say so without CLM inventing a convention (`reuse-first`). Such a land is set aside, neither green nor red. The search keeps a range of lands `lo..hi`, `hi` the red one (at first the red commit itself) and every land before `lo` in the range known green; it probes the land in `lo..hi` (`hi` excluded) not set aside that is nearest `lo + (hi - lo) / 2` (rounded down), the later of two equally near, moving `lo` past a green probe and `hi` to a red one, until no such land is left. When `lo` is `hi`, that land is the culprit. Otherwise every land from `lo` to `hi` but the last was set aside and any of them may hold the break, so the culprit is unknown, and stderr says, after the outcome line, `culprit unknown: [verify] check could not run on <n> lands, <short lo> to <short hi-1>, before the first red one, <short hi>`. The search assumes, as `git bisect` does, that a land after a red one stays red; to conclude it is unknown it must probe every land it set aside, each a checkout and a check that ends at once. The record's components are those whose `paths` hold a file the culprit's first-parent diff changes, or, with no culprit, a file any of the lands that may hold it change: those from `lo` to `hi` when the search left it unknown, and in `import` mode, which searches nothing, the covered lands. A land with no first parent (the repository's first commit) diffs against the empty tree. When no component's `paths` hold such a file (a land that changed only doc-only paths), the root component.

**What it stops.** A red record is a report: `clm intake` lists it for triage (`orchestrator.triage/intake-verify-failures`) and `clm land` warns a branch touching its components. Once the human takes it in as a `kind: bug` item with `verify_failures`, `clm land` holds code lands on the item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands, letting through meanwhile a branch that resolves another item holding the same component (`land/verify-stop`). A host's run never holds a land by itself (`land-locally`).

**The mutation pass** (`mutation-pass`, `mutation-after-land`). A marker says which test claims a requirement, and the land's check says the test passed; neither says the test would fail if the code were wrong. Mutation says that, and it is slow, so it runs here, after the land. CLM names no tool: the project's `[verify] mutate` command gets a plan and writes verdicts, as `[land] check` gets a selection, and it is unset unless the project sets it. A requirement is *due* when one of its citing tests was added, or changed in more than its markers, since the last pass, so a requirement that was only re-cited isn't judged: its tests are the ones an earlier pass saw, or none did, and a sample of mutants drawn from a whole component can't judge it either, since on 2026-11-14 seventeen mutants of `clm-coverage` were noticed by the tests of seven of its ten requirements often and by those of three almost never, all ten sound. The plan therefore hands the command the code the lands changed first, and a limit of 20 further mutants from the rest of the component, a constant (`yagni`). Tests that predate the pass are never judged (`backlog/tests-written-before-the-mutation-pass-were-never-judged`). The pass runs after the `verified` record is appended and the outcome line printed, and outside the check lock, so it can lose no verdict and hold no land; it keeps the verify tree's lock, so two passes never share the tree. A command that fails, or a pass killed, appends nothing, and the next pass starts from the same commit.

**Filing** (`files-unproven`). Verify pushes nothing to `main`, so a verdict reaches the tree as a backlog item, landed through `clm land` like any branch: adding a new item lands on checks. It is found from the ledger each time, not remembered, so a filing that failed is made by the next `clm verify --due` on a clone in `local` mode. A pipeline has the right to push only the ledger, so `pipeline` mode records and leaves the filing to such a clone.


## ADDED Requirements

### Requirement: mutation-pass
When the settings it follows (`settings-from-main`) set `[verify] mutate`, a `clm verify --due` or `clm verify --commit` in `local` or `pipeline` mode whose check passed SHALL, after appending the `verified` record and printing the outcome line, and holding the verify tree's lock but not `.clm/check.lock`, run the mutation pass on the commit `C` it verified. With `P` the commit the newest `mutated` record on `C`'s first-parent chain names, or the `verified` record's `since` when there is none, a requirement is *due* when, between the trees of `P` and `C`, a test citing it at its current version in `C` (`check.coverage/test-spans`, groups left out) was added, or its lines changed in more than their markers, or when a backlog item named it under `unproven` in `P` and none does in `C`; with no `P`, none is due. For the due requirements it SHALL write a plan file holding, for each in ID order, the line `requirement <id>`, then one line `code <path>` for each file its component owns in `C`, then `diff <path>` naming a file that holds `git diff <P> <C>` over those files when that is not empty, then one line `test <file>`, a tab and the test's name for each test citing it in `C`, then `limit 20`. It SHALL run the command through `sh -c` in the verify tree, with `CLM_MUTATE` naming the plan, `CLM_MUTATE_RESULT` naming an empty file, and `CLM_LAND_DIR` and the `[env]` variables as the check gets them, its output copied to stderr. From the result file it SHALL read one line per requirement: `<id> killed`, the tests failed for at least one mutant; `<id> survived <n>`, `<n>` mutants were tried and the tests passed for every one; or `<id> not-judged`; a due requirement with no such line, or with `survived 0`, is not judged. It SHALL then append one `mutated` record naming `C`, `P` as `since`, the killed and not-judged IDs and the survived ones as `unproven` entries at their versions in `C` (`ledger/mutated-record`), and print `mutation: <k> killed, <u> unproven, <n> not judged` on stderr. With nothing due it SHALL run no command and append the record with empty lists. When the command exits non-zero it SHALL append nothing and print `mutation: <command> failed with status <n>; nothing recorded` on stderr. The pass SHALL NOT change the outcome line, anything on stdout or the exit status, and SHALL NOT run when the check failed, in `import` mode, for a result answered from the ledger, for a bisect, or with `mutate` unset.

#### Scenario: a new test that notices
- GIVEN `mutate` set to a fake that copies its plan and writes `a/x killed`, a passing check, a `mutated` record for the last verified commit, and one land since that added `tests/x.rs` with a test `proves_x` citing `a/x` and changed `src/a.rs`, both `a`'s
- WHEN `clm verify --due` runs
- THEN the plan holds `requirement a/x`, `code src/a.rs`, `code tests/x.rs`, a `diff` line whose file names `src/a.rs`, `test tests/x.rs` with `proves_x` and `limit 20`; the ledger gains a `mutated` record with `a/x` under `killed`; stdout is the one line `verify passed <sha> 1 lands`; and it exits 0

#### Scenario: a test that notices nothing
- GIVEN the same with a fake that writes `a/x survived 12`
- WHEN `clm verify --due` runs
- THEN the `mutated` record holds `a/x@<version>` under `unproven`, stderr holds `mutation: 0 killed, 1 unproven, 0 not judged`, and it still prints `verify passed` and exits 0

#### Scenario: only re-cited
- GIVEN a land since the last pass that changed only the version in the marker of the test citing `a/x`
- WHEN `clm verify --due` runs
- THEN the command isn't run, and the `mutated` record's lists are empty

#### Scenario: an item resolved
- GIVEN a backlog item naming `a/x` under `unproven` at the last pass's commit, deleted by a land since, with the tests unchanged
- WHEN `clm verify --due` runs
- THEN the plan holds `requirement a/x`

#### Scenario: the command fails
- GIVEN a `mutate` command that exits 3
- WHEN `clm verify --due` runs with a passing check
- THEN no `mutated` record is appended, stderr names the command and status 3, and it prints `verify passed` and exits 0

#### Scenario: unset
- GIVEN a `[verify]` with no `mutate`
- WHEN `clm verify --due` runs with a passing check
- THEN no `mutated` record is appended and stderr holds no `mutation:` line

#### Scenario: a red check
- GIVEN `mutate` set and a check that fails
- WHEN `clm verify --due` runs
- THEN the command isn't run

#### Scenario: lands don't wait
- GIVEN a `mutate` command that waits for a file to appear
- WHEN `clm verify --due` is in its pass and a land of the clone runs its check
- THEN the land's check starts before the file appears

#### Serves
- architecture/proven-by-tests
- capability/prove-by-tests

#### Needs
- config/verify-setting
- ledger/mutated-record
- check.coverage/test-spans
- model/parse-backlog

### Requirement: files-unproven
At the end of every `clm verify --due` in `local` mode whose settings set `[verify] mutate`, whether or not a run was due, and after the mutation pass when one ran, it SHALL take `Fold::unfiled_unproven` over `origin/main`'s first-parent chain (`ledger/mutated-record`) and keep the entries that name a requirement at its current version in `origin/main` and that no backlog item on `origin/main` names under `unproven`. When any is left it SHALL, in a temporary worktree on a branch `unproven/<short sha>` cut from `origin/main`, `<short sha>` being `origin/main`'s, write one item per component, `backlog/unproven-<component>-<short sha>.md`, with `title` `Tests of <component> notice no change to its code`, `kind: bug`, `components` that component and `unproven` its entries, and a body naming, for each entry, its citing tests and the commit of the record that found it; commit them; push the branch and land it with `clm land`, as any branch lands; and delete the branch, here and on `origin`, whether or not it landed. It SHALL print `unproven: filed <n> items for <m> requirements` on stderr when it landed, and `unproven: not filed: <reason>` when the push or the land failed, leaving the entries for the next call. Neither SHALL change the outcome line, stdout or the exit status.

#### Scenario: filed and read as not built
- GIVEN a `mutated` record for `origin/main`'s head with `unproven` `a/x@<current version>`, and `a/x` current
- WHEN `clm verify --due` runs with nothing due, and then `clm coverage` runs on the new `origin/main`
- THEN `origin/main` holds `backlog/unproven-a-<short sha>.md`, a `bug` item on `a` with `unproven: [a/x@<version>]`, no branch `unproven/<short sha>` is left on `origin`, and coverage prints `a/x unbuilt`

#### Scenario: already filed
- GIVEN the same once the item is on `origin/main`
- WHEN `clm verify --due` runs again
- THEN it lands nothing

#### Scenario: reworded since
- GIVEN a `mutated` record with `unproven` `a/x@11111111`, and `a/x` at another version on `origin/main`
- WHEN `clm verify --due` runs
- THEN it lands nothing

#### Scenario: killed by a later pass
- GIVEN that record, and a later one naming `a/x` under `killed`
- WHEN `clm verify --due` runs
- THEN it lands nothing

#### Scenario: the land fails
- GIVEN an entry to file and an `origin` that refuses the push
- WHEN `clm verify --due` runs
- THEN stderr holds `unproven: not filed:` with the reason, the exit status is what the verification gave, and a later call files it

#### Scenario: pipeline mode
- GIVEN `mode = "pipeline"` and an entry to file
- WHEN `clm verify --due` runs
- THEN it lands nothing

#### Serves
- architecture/proven-by-tests
- capability/prove-by-tests

#### Needs
- ledger/mutated-record
- model/parse-backlog
- check.coverage/coverage-report
