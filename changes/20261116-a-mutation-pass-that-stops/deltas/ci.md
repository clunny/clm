---
component: ci
---

## MODIFIED Requirements

### Requirement: mutate-script <!-- base: 28ff1a918aa107b3 -->
`.github/scripts/mutate.sh` SHALL be a command for `[verify] mutate` (`verify/mutation-pass`): run from the repository root with `CLM_MUTATE` naming a plan and `CLM_MUTATE_RESULT` a file, it SHALL write one line for each `requirement <id>` block of the plan, appending each block's line to the result file in one write, ending in a newline, as soon as the block is judged, so a pass ended part-way leaves every verdict it reached (`verify/mutation-pass`). It SHALL judge up to `CLM_MUTATE_JOBS` blocks at once, or one when that is unset or not a positive integer, starting them in the plan's order, after every tool it needs is installed and every fixture the plan's blocks need is judged; each block it runs SHALL have a job number from 1 up to that count that no block running beside it holds, a `CARGO_TARGET_DIR` of its own for that job number for its `cargo llvm-cov` run and its `cargo mutants` calls, and `CLM_SLOT` set, for every command it runs, to the `CLM_SLOT` it was given plus its job number less one, while the fixtures and `coverage.sh` run with the `CLM_SLOT` it was given, so blocks at once share no build directory and no port or path their tests derive from the slot (`tests-run-concurrently`). On `SIGTERM` it SHALL end the blocks it runs and remove its temporary directory before it exits. For a block whose `code` lines name Rust files under `crates/`, it SHALL run `cargo mutants` at a version the script pins, installed with `cargo install --locked --version <x.y.z> cargo-mutants` when absent, on the mutants of those files that `--in-diff` on the block's `diff` file finds when it has one, and, when it tries none of those or the tests pass for every one, on up to the block's `limit` further mutants of those files not among them, in `cargo mutants --list` order, the limit counting only mutants it may try. It SHALL try a mutant of either kind only when a line from the start to the end of its span, as `cargo mutants --list --json` gives them, is one the block's tests run, since a mutant of code the requirement's tests never run is one no test citing it could notice, and SHALL run the mutants it tries by name (`--re`). It SHALL learn those lines after the fixture and before the block's first mutant, from one run of the block's tests alone in the tree it makes, under the same filterset, through `cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --lcov` over the block's test packages, with `cargo-llvm-cov` at the version `coverage.sh` pins, installed as `coverage.sh` installs it when absent, reading each `SF:` path relative to that tree. When the block's tests run no line of any of its files that has a mutant, as an empty test does, it SHALL try no mutant and count as survived the diff's mutants and the first `limit` further ones, since tests that run none of the code can notice no change to it; when they run lines of those files but none a mutant changes, as when the requirement's rule lives in a data or text file that no mutation tool changes, it SHALL try none and write `not-judged`; it SHALL run `cargo mutants` in a tree it makes once, before its first Rust block, in its temporary directory: a git repository whose `.git` is a directory of its own, sharing the checkout's object store by an absolute path (`git clone --shared`) and holding none of it, checked out at the checkout's `HEAD` commit, with the checkout's branches and remote-tracking branches under the same names, and every tracked file marked unchanged in its index (`git update-index --assume-unchanged`); and it SHALL have `cargo mutants` copy that `.git` into each copy it tests (`--copy-vcs true`), so that a test that asks git about this repository passes there as it does in the checkout, a mutated file leaves git reporting no tracked file changed from `HEAD`, and no copy reads or writes the checkout's index, refs or git directory; the tests it runs for each mutant SHALL be the block's `test` lines alone, each given to nextest as a filterset naming its package, its test target by nextest's binary ID, `binary_id(=<package>::<target>)`, where `ci-check.sh` passes `--test <target>` for a selected file, and its name. For a block whose `code` lines name TypeScript files under `web/src/` or `plugin-panel/`, it SHALL run StrykerJS with its vitest runner at a pinned version, added to `node_modules/` alone with `npm install --no-save --ignore-scripts` so no `package.json` or lockfile changes, mutating the diff's lines first and then up to `limit` more, with the block's vitest `test` lines alone; it SHALL give StrykerJS its test runner, the vitest configuration that names those tests, its reporter and its `mutate` ranges in a configuration file it writes under that directory's `node_modules/`, since StrykerJS takes a runner's options from no command line, and that file SHALL set no `thresholds.break`, so StrykerJS's exit status never depends on a score. It SHALL count a mutant StrykerJS reports `NoCoverage`, one the block's tests never run, as neither caught nor survived, unless every mutant StrykerJS reports for the block is `NoCoverage`, when it SHALL count them all as survived, as for an empty test. It SHALL write `<id> killed` when the tests failed for a mutant, `<id> survived <n>` when `<n>` mutants built and the tests passed for every one, and `<id> not-judged` when no mutant built, when the block's tests run lines of its files but none a mutant changes, when the block's tests are all Playwright specs, or when its `code` lines name neither kind of file. After the blocks it SHALL run `.github/scripts/coverage.sh` and, when `CLM_LAND_DIR` is set, copy `target/lcov.info` to `$CLM_LAND_DIR/lcov.info`. Before the first block that needs a tool, it SHALL judge a fixture it writes itself with that tool, a function with one test that fails when the function is changed and one that passes whatever it returns: for `cargo-mutants` a crate in a temporary directory that the script makes a git repository with its files committed and from which it makes a tree the way it makes the checkout's, and whose tests pass only inside a git repository whose top is the crate and whose blind test also fails when git reports a tracked file changed from `HEAD` (`git status --porcelain --untracked-files=no`), so that a tool that leaves `.git` out of its copy, or a copy in which git sees the mutated file, fails the fixture, once, and for StrykerJS a source file and a test file inside the npm directory of the block, once for each such directory before its first block, run with that directory's installed vitest and TypeScript, both removed afterwards; it SHALL go on only when the tool reports the first test's mutants caught and the second's missed. It SHALL exit non-zero, naming the tool and the block or the fixture on stderr and writing no verdict for that block, when a tool can't be installed, when a tool's verdict on the fixture is wrong, when a tool fails for a reason other than its mutants' outcomes (`cargo mutants` exiting with a status other than 0, 2 or 3, as when the tests fail in the unmutated tree or a filterset doesn't parse; StrykerJS exiting non-zero, writing no report or running no test), when the run that finds the lines a block's tests run fails or writes no report, or when `coverage.sh` fails; and 0 otherwise, whatever the verdicts. When a block fails so, it SHALL start no further block, let the blocks already running finish and write their lines, run no `coverage.sh`, and then exit non-zero. So a tool that doesn't work records no verdict for the block it failed in, and that block and every block not started stay due (`verify/mutation-pass`). No workflow, and neither `ci-check.sh` nor `clm.toml`'s `[land] check`, SHALL run it. Its tests put fakes of `cargo`, `rustup` and `npx` on `PATH`, as `coverage-script`'s do, so they install and mutate nothing; the fake `cargo llvm-cov` reports every line of the block's files run unless a scenario says otherwise.

#### Scenario: a Rust requirement whose test notices
- GIVEN a plan with one block for a requirement of a crate, with a `diff` and one `test` line, and a fake `cargo` whose `mutants` reports one mutant caught
- WHEN `mutate.sh` runs
- THEN the fake was called with `--in-diff` and a filterset holding `package(=<package>)`, `binary_id(=<package>::<target>)` and the test's name, no `binary(` among its words, and the result file holds `<id> killed`

#### Scenario: tests that notice nothing
- GIVEN the same with a fake `cargo llvm-cov` that reports every line of the block's files run, and a fake `mutants` that reports 3 mutants missed for the diff and 2 more missed for the files
- WHEN it runs
- THEN the result file holds `<id> survived 5`

#### Scenario: a tool that fails
- GIVEN a plan with one Rust block, and a fake `cargo` whose `mutants` exits 4, as it does when the tests fail in an unmutated tree
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo mutants` and the requirement, and the result file holds no line for it

#### Scenario: StrykerJS is given a configuration file
- GIVEN a plan with one block for a requirement of `web/src/`, with a `diff` and one vitest `test` line, and a fake `npx` whose `stryker run` records its arguments and the file they name, and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN `stryker run` was given a configuration file under `web/node_modules/` that names the vitest runner, a vitest configuration including that test alone and the diff's ranges of the block's files; no argument begins `--vitest`; and the result file holds `<id> killed`

#### Scenario: StrykerJS runs no test
- GIVEN the same with a fake `npx` whose `stryker run` exits 1 and writes no report
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names StrykerJS and the requirement, and the result file holds no line for it

#### Scenario: the fixture first
- GIVEN a plan with one Rust block, and a fake `cargo` that records each call
- WHEN `mutate.sh` runs
- THEN the first `cargo mutants` call ran in a crate outside the repository, before any call naming the block's files, and no file of the repository changed

#### Scenario: a tool that judges the fixture wrongly
- GIVEN the same with a fake `cargo` whose `mutants` reports every mutant caught, the fixture's blind test included
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming `cargo mutants` and the fixture, no call named the block's files, and the result file is empty

#### Scenario: Playwright only
- GIVEN a block whose `test` lines all name files under `web/e2e/`
- WHEN it runs
- THEN the result file holds `<id> not-judged`, and neither tool, nor its fixture, ran

#### Scenario: not in the land check
- GIVEN `.github/scripts/ci-check.sh`, the workflows under `.github/workflows/` and `clm.toml`'s `[land]`
- WHEN they are read
- THEN none names `mutate.sh`, `cargo mutants` or `stryker`

#### Scenario: a test that asks git about the repository
- GIVEN a plan whose block names a Rust file of a crate and a test of that crate that passes only inside a git repository, and a stand-in `cargo mutants` that records its arguments
- WHEN the script runs for the block
- THEN the recorded call holds `--copy-vcs true`

#### Scenario: the fixture is made as the tree is
- GIVEN a plan with one Rust block, and a stand-in `cargo mutants` that records its arguments and the directory it ran in
- WHEN the script runs
- THEN when the tool is called in the fixture's directory, its `.git` is a directory whose `objects/info/alternates` names another repository's object store, `git status --porcelain --untracked-files=no` there prints nothing after its `src/lib.rs` is changed, the call holds `--copy-vcs true`, and the fixture's test source asks git for the repository's top and for the working tree's status

#### Scenario: the tree the tool copies has a git directory of its own
- GIVEN a plan with one Rust block, run in a linked worktree of a repository that has a branch and a remote-tracking branch, and a stand-in `cargo mutants` that records the directory it ran in
- WHEN the script runs for the block
- THEN that directory is outside the checkout, its `.git` is a directory with no `objects/pack/*.pack`, `git rev-parse HEAD` there prints the checkout's `HEAD`, the branch and the remote-tracking branch resolve there to the commits they name in the checkout, `git status --porcelain --untracked-files=no` there prints nothing after a tracked file there is changed, and the checkout's index file holds the same bytes as before the run

#### Scenario: a mutant the tests never run is not tried
- GIVEN a plan with one Rust block with no `diff` and one `test` line, a fake `cargo` whose `mutants --list --json` lists two mutants of the block's file, one spanning line 3 alone and one line 30 alone, whose `llvm-cov` reports line 3 run and line 30 not, and whose `mutants` reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN the `llvm-cov` call's filterset names the block's test, the `cargo mutants` call that tries the block's mutants names the mutant on line 3 and not the one on line 30, and the result file holds `<id> survived 1`

#### Scenario: tests that run none of the code
- GIVEN the same with a fake `llvm-cov` that reports no line of the block's file run
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> survived 2`

#### Scenario: tests that run the code but no mutant of it
- GIVEN the same with a fake `llvm-cov` that reports line 10 of the block's file run and lines 3 and 30 not
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> not-judged`

#### Scenario: the run that finds the lines fails
- GIVEN the same with a fake `llvm-cov` that exits 1
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo llvm-cov` and the requirement, and the result file holds no line for it

#### Scenario: a mutant StrykerJS finds no test running
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, and a fake `npx` whose `stryker run` reports one mutant `Survived` and two `NoCoverage`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 1`

#### Scenario: StrykerJS finds no test running any mutant
- GIVEN the same with a fake `npx` whose `stryker run` reports two mutants, both `NoCoverage`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 2`

#### Scenario: blocks at once
- GIVEN a plan with three Rust blocks, `CLM_MUTATE_JOBS=2`, `CLM_SLOT=5`, and a stand-in `cargo` whose `mutants` and `llvm-cov` record, for each call, its start and end, `CLM_SLOT` and `CARGO_TARGET_DIR`, and wait until a second block's call runs or two seconds pass
- WHEN `mutate.sh` runs
- THEN calls for two different blocks ran at once, calls for three never did, every block's call ran with `CLM_SLOT` 5 or 6, two blocks' calls running at once never shared a slot or a `CARGO_TARGET_DIR`, the fixture's calls ran with `CLM_SLOT` 5 and all ended before the first block's began, and the result file holds three lines

#### Scenario: each line as it is judged
- GIVEN a plan with two Rust blocks, `CLM_MUTATE_JOBS` unset, and a stand-in `cargo` whose `mutants` reports the first block's mutant caught and, for the second block, waits until a file appears
- WHEN `mutate.sh` runs
- THEN the result file holds the first block's `killed` line, ending in a newline, before the file appears

#### Scenario: a failing block lets the others finish
- GIVEN a plan with three Rust blocks, `CLM_MUTATE_JOBS=2`, and a stand-in `cargo` whose `mutants` exits 4 for the first block and, for the second, reports a mutant caught after the first has failed
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming `cargo mutants` and the first block's requirement, the result file holds the second block's `killed` line and no line for the first or the third, no call named the third block's files, and `coverage.sh` didn't run

#### Scenario: ended part-way
- GIVEN a plan with two Rust blocks and a stand-in `cargo` whose `mutants` reports the first block's mutant caught and, for the second, sleeps
- WHEN `mutate.sh` gets `SIGTERM` once the first line is written
- THEN it exits, the stand-in no longer runs, the result file holds the first block's line alone, and the temporary directory it made is gone

#### Serves
- architecture/proven-by-tests
- architecture/land-checks-stay-fast

#### Needs
- verify/mutation-pass
