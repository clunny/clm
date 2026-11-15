---
component: verify
---

## MODIFIED Requirements

### Requirement: mutation-pass <!-- base: e561e63faca60907 -->
When the settings it follows (`settings-from-main`) set `[verify] mutate`, a `clm verify --due` or `clm verify --commit` in `local` or `pipeline` mode whose check passed SHALL, after appending the `verified` record and printing the outcome line, and holding the verify tree's lock but not `.clm/check.lock`, run the mutation pass on the commit `C` it verified. With `P` the commit the newest `mutated` record on `C`'s first-parent chain names, or, when there is none, the first parent of the oldest commit of the unbroken run of commits ending at `C` on that chain whose `clm.toml` sets `[verify] mutate`, which is the last commit before the project switched the pass on, a requirement is *due*; a commit sets it when its `clm.toml` parses as TOML and holds a non-empty string there, and any other commit ends the run when, between the trees of `P` and `C`, a test citing it at its current version in `C` (`check.coverage/test-spans`, groups left out) was added, or its lines changed in more than their markers; with no `P`, as when the chain's first commit already sets `mutate`, every requirement a test of `C` cites at its current version is due. So a pass that fails before any has been recorded loses nothing: the next one starts from the same commit. For the due requirements it SHALL write a plan file holding, for each in ID order, the line `requirement <id>`, then one line `code <path>` for each file its component owns in `C`, then `diff <path>` naming a file that holds `git diff <P> <C>` over those files when that is not empty, then one line `test <file>`, a tab and the test's name for each test citing it in `C`, then `limit 20`. It SHALL run the command through `sh -c` in the verify tree, with `CLM_MUTATE` naming the plan, `CLM_MUTATE_RESULT` naming an empty file, `CLM_SLOT` set to one more than the clone's `max_builds`, a number no build place and no check holds, so the tests it runs may overlap any other run (`tests-run-concurrently`), and `CLM_LAND_DIR` and the `[env]` variables as the check gets them, its output copied to stderr. From the result file it SHALL read one line per requirement: `<id> killed`, the tests failed for at least one mutant; `<id> survived <n>`, `<n>` mutants were tried and the tests passed for every one; or `<id> not-judged`; a due requirement with no such line, or with `survived 0`, is not judged. It SHALL then append one `mutated` record naming `C`, `P` as `since`, the killed and not-judged IDs and the survived ones as `unproven` entries at their versions in `C` (`ledger/mutated-record`), and print `mutation: <k> killed, <u> unproven, <n> not judged` on stderr. With nothing due it SHALL run no command and append the record with empty lists. When the command exits non-zero it SHALL append nothing and print `mutation: <command> failed with status <n>; nothing recorded` on stderr. The pass SHALL NOT change the outcome line, anything on stdout or the exit status, and SHALL NOT run when the check failed, in `import` mode, for a result answered from the ledger, for a bisect, or with `mutate` unset.

#### Scenario: a new test that notices
- GIVEN `mutate` set to a fake that copies its plan and writes `a/x killed`, a passing check, a `mutated` record for the last verified commit, and one land since that added `tests/x.rs` with a test `proves_x` citing `a/x` and changed `src/a.rs`, both `a`'s
- WHEN `clm verify --due` runs
- THEN the plan holds `requirement a/x`, `code src/a.rs`, `code tests/x.rs`, a `diff` line whose file names `src/a.rs`, `test tests/x.rs` with `proves_x` and `limit 20`; the ledger gains a `mutated` record with `a/x` under `killed`; stdout is the one line `verify passed <sha> 1 lands`; and it exits 0

#### Scenario: a test that notices nothing
- GIVEN the same with a fake that writes `a/x survived 12`
- WHEN `clm verify --due` runs
- THEN the `mutated` record holds `a/x@<version>` under `unproven`, stderr holds `mutation: 0 killed, 1 unproven, 0 not judged`, and it still prints `verify passed` and exits 0

#### Scenario: the first pass fails, and the next loses nothing
- GIVEN no `mutated` record, a land `S` that set `mutate`, a land after it that added a test citing `a/x`, a `mutate` command that exits 3, and `clm verify --due` run once, which records the verification and no pass
- WHEN a further land adds a test citing `a/y`, the command is replaced by one that copies its plan, and `clm verify --due` runs again
- THEN the plan holds blocks for `a/x` and `a/y`, and the `mutated` record's `since` is the first parent of `S`

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
