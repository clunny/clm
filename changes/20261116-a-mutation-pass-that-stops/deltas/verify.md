---
component: verify
---

## MODIFIED Requirements

### Requirement: mutation-pass <!-- base: cf9b872be3e00a71 -->
When the settings it follows (`settings-from-main`) set `[verify] mutate`, a `clm verify --due` or `clm verify --commit` in `local` or `pipeline` mode whose check passed SHALL, after appending the `verified` record and printing the outcome line, and holding the verify tree's lock but not `.clm/check.lock`, run the mutation pass on the commit `C` it verified. With `P` the commit the newest `mutated` record on `C`'s first-parent chain names, or, when there is none, the first parent of the oldest commit of the unbroken run of commits ending at `C` on that chain whose `clm.toml` sets `[verify] mutate`, which is the last commit before the project switched the pass on, a requirement is *due*; a commit sets it when its `clm.toml` parses as TOML and holds a non-empty string there, and any other commit ends the run when, between the trees of `P` and `C`, a test citing it at its current version in `C` (`check.coverage/test-spans`, groups left out) was added, or its lines changed in more than their markers; with no `P`, as when the chain's first commit already sets `mutate`, every requirement a test of `C` cites at its current version is due. A requirement the newest `mutated` record on that chain names under `left` is due as well, when a test of `C` cites it at its current version, so a pass that ends before it judges a requirement loses nothing: the next one judges it, and a pass that fails before recording any record starts from the same commit. For the due requirements it SHALL write a plan file holding, for each, first those under that record's `left` in the list's order and then the others in ID order, the line `requirement <id>`, then one line `code <path>` for each file its component owns in `C`, then `diff <path>` naming a file that holds `git diff <P> <C>` over those files when that is not empty, then one line `test <file>`, a tab and the test's name for each test citing it in `C`, then `limit 20`. It SHALL run the command through `sh -c` in the verify tree, with `CLM_MUTATE` naming the plan, `CLM_MUTATE_RESULT` naming an empty file, `CLM_SLOT` set to one more than the clone's `max_builds`, a number that, with the `mutate_jobs` less one numbers after it, no build place and no check holds, so the tests it runs may overlap any other run (`tests-run-concurrently`), `CLM_MUTATE_JOBS` set to the whole number of 2 GiB in the `MemAvailable` line of `/proc/meminfo`, at least 1 and at most `[verify] mutate_jobs`, or to 1 when `mutate_jobs` is unset or that line can't be read, and `CLM_LAND_DIR` and the `[env]` variables as the check gets them, its output copied to stderr, in a process group of its own. With `[verify] mutate_minutes` set, when the command has run that many minutes it SHALL send `SIGTERM` to that group, and `SIGKILL` 10 seconds later when the command hasn't ended, and print `mutation: stopped after <m> minutes` on stderr; when `clm verify` gets `SIGINT` or `SIGTERM` while the command runs, it SHALL end the command the same way and print `mutation: stopped` on stderr. However the command ends, it SHALL then read from the result file the lines written, one per requirement, ignoring a line that doesn't parse: `<id> killed`, the tests failed for at least one mutant; `<id> survived <n>`, `<n>` mutants were tried and the tests passed for every one; or `<id> not-judged`; a line of `survived 0` is not judged, and a due requirement with no line is left. It SHALL then append one `mutated` record naming `C`, `P` as `since`, the killed and not-judged IDs, the survived ones as `unproven` entries at their versions in `C`, and the left ones under `left` in the plan's order (`ledger/mutated-record`), and print `mutation: <k> killed, <u> unproven, <n> not judged` on stderr, followed by `, <l> left` when any is left. With nothing due it SHALL run no command and append the record with empty lists. When the command exits non-zero without being ended so, it SHALL print `mutation: <command> failed with status <n>` on stderr before the record's line, and still append the record, so the verdicts it reached are kept and the rest are left. The pass SHALL NOT change the outcome line, anything on stdout or the exit status, except that `clm verify` stopped by a signal exits, once the record is appended, with 128 plus the signal's number, and SHALL NOT run when the check failed, in `import` mode, for a result answered from the ledger, for a bisect, or with `mutate` unset.

#### Scenario: a new test that notices
- GIVEN `mutate` set to a fake that copies its plan and writes `a/x killed`, a passing check, a `mutated` record for the last verified commit, and one land since that added `tests/x.rs` with a test `proves_x` citing `a/x` and changed `src/a.rs`, both `a`'s
- WHEN `clm verify --due` runs
- THEN the plan holds `requirement a/x`, `code src/a.rs`, `code tests/x.rs`, a `diff` line whose file names `src/a.rs`, `test tests/x.rs` with `proves_x` and `limit 20`; the ledger gains a `mutated` record with `a/x` under `killed`; stdout is the one line `verify passed <sha> 1 lands`; and it exits 0

#### Scenario: a test that notices nothing
- GIVEN the same with a fake that writes `a/x survived 12`
- WHEN `clm verify --due` runs
- THEN the `mutated` record holds `a/x@<version>` under `unproven`, stderr holds `mutation: 0 killed, 1 unproven, 0 not judged`, and it still prints `verify passed` and exits 0

#### Scenario: the first pass fails, and the next loses nothing
- GIVEN no `mutated` record, a land `S` that set `mutate`, a land after it that added a test citing `a/x`, a `mutate` command that writes nothing and exits 3, and `clm verify --due` run once, which records a `mutated` record whose `since` is the first parent of `S` and whose `left` holds `a/x`
- WHEN a further land adds a test citing `a/y`, the command is replaced by one that copies its plan, and `clm verify --due` runs again
- THEN the plan holds the block for `a/x` and then the one for `a/y`, and the new `mutated` record's `since` is the commit the first one named

#### Scenario: the pass switched on from the first commit
- GIVEN no `mutated` record and a chain whose first commit sets `mutate` and holds a test citing `a/x` at its current version
- WHEN `clm verify --due` runs with a passing check
- THEN the plan holds a block for `a/x`, with no `diff` line

#### Scenario: only re-cited
- GIVEN a land since the last pass that changed only the version in the marker of the test citing `a/x`
- WHEN `clm verify --due` runs
- THEN the command isn't run, and the `mutated` record's lists are empty

#### Scenario: an item resolved with its test unchanged
- GIVEN a backlog item naming `a/x` under `unproven` at the last pass's commit, deleted by a land since, with the tests unchanged
- WHEN `clm verify --due` runs
- THEN the command isn't run for `a/x`, so a build that found the test sound settles it in one look

#### Scenario: its own slot
- GIVEN `max_builds` 4 and a fake `mutate` that records `CLM_SLOT`
- WHEN the pass runs it
- THEN it recorded `5`

#### Scenario: the command fails part-way
- GIVEN `a/x`, `a/y` and `a/z` due, and a `mutate` command that writes `a/x killed` and exits 3
- WHEN `clm verify --due` runs with a passing check
- THEN the `mutated` record holds `a/x` under `killed` and `a/y` and `a/z` under `left`, stderr names the command and status 3 and then holds `mutation: 1 killed, 0 unproven, 0 not judged, 2 left`, and it prints `verify passed` and exits 0

#### Scenario: a line cut short
- GIVEN `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed`, a newline and `a/y kil` and exits 3
- WHEN `clm verify --due` runs
- THEN the record holds `a/x` under `killed` and `a/y` under `left`

#### Scenario: the time limit
- GIVEN `mutate_minutes = 1`, `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed`, starts a child that sleeps, and then sleeps itself
- WHEN `clm verify --due` runs
- THEN after a minute the command and its child no longer run, the record holds `a/x` under `killed` and `a/y` under `left`, stderr holds `mutation: stopped after 1 minutes`, and it prints `verify passed` and exits 0

#### Scenario: stopped by hand
- GIVEN `a/x` and `a/y` due, and a `mutate` command that writes `a/x killed` and then sleeps
- WHEN `clm verify --due` gets `SIGTERM` during the pass
- THEN the command no longer runs, the record holds `a/x` under `killed` and `a/y` under `left`, stderr holds `mutation: stopped`, stdout holds `verify passed`, and it exits 143

#### Scenario: what is left goes first
- GIVEN the newest `mutated` record's `left` holding `a/z`, and a land since that added a test citing `a/a`
- WHEN `clm verify --due` runs with a fake that copies its plan
- THEN the plan's block for `a/z` comes before the one for `a/a`

#### Scenario: jobs follow memory
- GIVEN `mutate_jobs = 6` and a fake `mutate` that records `CLM_MUTATE_JOBS`
- WHEN the pass runs where `MemAvailable` reads 5 GiB, then where it reads 40 GiB, then with `mutate_jobs` unset
- THEN it recorded `2`, then `6`, then `1`

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
