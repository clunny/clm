---
component: ci
---

## MODIFIED Requirements

### Requirement: mutate-script <!-- base: c1e5945adccc49c0 -->
`.github/scripts/mutate.sh` SHALL be a command for `[verify] mutate` (`verify/mutation-pass`): run from the repository root with `CLM_MUTATE` naming a plan and `CLM_MUTATE_RESULT` a file, it SHALL empty the result file and then write one line for each `requirement <id>` block of the plan, appending each block's line to the result file in one write, ending in a newline, as soon as the block is judged, so a pass ended part-way leaves every verdict it reached (`verify/mutation-pass`). It SHALL judge up to `CLM_MUTATE_JOBS` blocks at once, or one when that is unset or not a positive integer, starting them in the plan's order, after every tool it needs is installed and every fixture the plan's blocks need is judged; each block it runs SHALL have a job number from 1 up to that count that no block running beside it holds, a `CARGO_TARGET_DIR` of its own for that job number for its `cargo llvm-cov` runs, its `cargo mutants` calls and its `cargo nextest` runs, and `CLM_SLOT` set, for every command it runs, to the `CLM_SLOT` it was given plus its job number less one, while the fixtures and `coverage.sh` run with the `CLM_SLOT` it was given, so blocks at once share no build directory and no port or path their tests derive from the slot (`tests-run-concurrently`). With `CLM_MUTATE` or `CLM_MUTATE_RESULT` unset or empty, or `CLM_SLOT` set to what isn't a whole number, it SHALL exit non-zero, naming the variable on stderr (`mutate.sh: CLM_SLOT is not a whole number: <value>` for `CLM_SLOT`), before it runs any tool or empties the result file. On `SIGTERM` or `SIGINT` it SHALL end the blocks it runs and what they started, remove its temporary directory, and exit with status 143. When `cargo nextest --version` fails it SHALL exit non-zero before its first Rust block, naming `cargo-nextest` on stderr, since it installs no nextest. For a block whose `code` lines name Rust files under `crates/`, it SHALL run `cargo mutants` at a version the script pins, installed with `cargo install --locked --version <x.y.z> cargo-mutants` when absent, on the mutants of those files that `--in-diff` on the block's `diff` file finds when it has one, and, when it tries none of those or the tests pass for every one, on up to the block's `limit` further mutants of those files not among them, the limit counting only mutants it may try, taken by their share, highest first, and in `cargo mutants --list` order among equal shares. A line's share is its count in the run that finds the lines the block's tests run (below) divided by the sum of its counts in the runs of every test of the block's test packages, or 1 when that sum is 0 or the quotient is larger; a mutant's share is the highest share among the lines of its span the block's tests run. A line's count in a report is the sum of the counts its `DA:` lines give it there. So the mutants of code only the requirement's tests run come first, and those of code every test of the package runs, which no test citing the requirement is about, come last. It SHALL make the run of every test of a package only when more further mutants may be tried than the block's `limit`, after the run that finds the lines the block's tests run, in the tree it makes, through `cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --ignore-run-fail --lcov` over that package with no filterset, reading each `SF:` path relative to that tree, writing its report to a path of the block's job and only then keeping it under the package's name for the rest of the pass, so a block that starts after it was kept runs none of its own for that package and two blocks at once never write one report. It SHALL try a mutant of either kind only when a line from the start to the end of its span, as `cargo mutants --list --json` gives them, is one the block's tests run, since a mutant of code the requirement's tests never run is one no test citing it could notice, and SHALL run the mutants it tries by name (`--re`). It SHALL learn those lines after the fixture and before the block's first mutant, from one run of the block's tests alone in the tree it makes, under the same filterset, through `cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --lcov` over the block's test packages, with `cargo-llvm-cov` at the version `coverage.sh` pins, installed as `coverage.sh` installs it when absent, reporting the lines of files under a crate's `tests/` directory too, which `cargo llvm-cov` leaves out by default, and reading each `SF:` path relative to that tree. When the block's tests run no line of any of its files that has a mutant, or run lines of those files but none a mutant changes, and they run no line of any other Rust file of the repository but the files of its `test` lines, so that a test that runs another component's code still reads as before, and a file of the block's `test` lines under a crate's `tests/` directory, a test target's file, which `cargo mutants` never mutates, has a mutant that `cargo mutants --Zmutate-file <file> --json`, given the tree's copy of that file, lists on a line from the start to the end of whose span the block's tests run, it SHALL judge the block by those mutants instead, since its rule may be code in its test's own file, as a scan of the repository's sources is: up to the block's `limit` of them, file by file in the order of the block's `test` lines and each file's in the tool's order, trying none after the first one caught, the block's `diff` playing no part; it SHALL write each into the file, as the mutant's `diff` changes it, in a copy of the tree of its own for the block's job number, never the tree `cargo mutants` copies or the checkout, run the block's tests there with `cargo nextest run` under the block's filterset, and put the file back before the next and before it exits, counting the mutant caught when nextest exits 100, as when a test fails, missed when it exits 0, and neither when it exits 101, as when the mutated file doesn't build; it SHALL judge the block by them only then, never beside mutants of its code files, since a mutant of a helper that builds a test's fixture is caught however little the test asserts, and would hide the code's survivors. Otherwise, when the block's tests run no line of any of its files that has a mutant, as an empty test does, it SHALL try no mutant and count as survived the diff's mutants and the first `limit` further ones, since tests that run none of the code can notice no change to it, unless the block holds a `reads` line, when it SHALL try none and write `not-judged`, since a test that reads the repository's own files may prove a rule that is data in them (`verify/mutation-pass`); when they run lines of those files but none a mutant changes, as when the requirement's rule lives in a data or text file that no mutation tool changes, it SHALL try none and write `not-judged`; it SHALL run `cargo mutants` in a tree it makes once, before its first Rust block, in its temporary directory: a git repository whose `.git` is a directory of its own, sharing the checkout's object store by an absolute path (`git clone --shared`) and holding none of it, checked out at the checkout's `HEAD` commit, with the checkout's branches and remote-tracking branches under the same names, and every tracked file marked unchanged in its index (`git update-index --assume-unchanged`); and it SHALL have `cargo mutants` copy that `.git` into each copy it tests (`--copy-vcs true`), so that a test that asks git about this repository passes there as it does in the checkout, a mutated file leaves git reporting no tracked file changed from `HEAD`, and no copy reads or writes the checkout's index, refs or git directory; the tests it runs for each mutant SHALL be the block's `test` lines alone, each given to nextest as a filterset naming its package, its test target by nextest's binary ID, `binary_id(=<package>::<target>)`, where `ci-check.sh` passes `--test <target>` for a selected file, and its name. For a block whose `code` lines name TypeScript files under `web/src/` or `plugin-panel/`, it SHALL run StrykerJS with its vitest runner at a pinned version, added to `node_modules/` alone with `npm install --no-save --ignore-scripts` so no `package.json` or lockfile changes, mutating the diff's lines first and then up to `limit` more, with the block's vitest `test` lines alone, and SHALL give it no range of a file whose name ends in `.test.ts` or `.test.tsx`, a vitest test file, since StrykerJS's instrumentation of a test file moves a `vi.mock` call that vitest hoists out of the module's top level and fails its first run; it SHALL run StrykerJS from that directory with the directory its sandbox is made in (`tempDirName`, a directory under that directory) holding, before StrykerJS starts, a symbolic link for each entry of the repository's top but `.git` and that directory, to that entry, so that the sandbox stands where that directory does among the repository's other files and a path a test resolves from its own file to a file of the repository outside that directory, as `../../specs/web/SPEC.md` from `web/src/`, reaches that file in the sandbox as it does in the checkout, and it SHALL remove that directory as soon as StrykerJS returns, so no later run, such as `coverage.sh`'s vitest, finds the links; it SHALL have StrykerJS copy into its sandbox no file or directory under that directory that the repository's `.gitignore` files mark ignored, the top's and each one under that directory, whether it exists when StrykerJS starts or appears while StrykerJS copies, so that output another run makes and removes there, as the site's tests do `web/site/.test-dist/`, is never in the sandbox and can't fail StrykerJS's copy; it SHALL give StrykerJS its test runner, the vitest configuration that names those tests, its reporter and its `mutate` ranges in a configuration file it writes under that directory's `node_modules/`, since StrykerJS takes a runner's options from no command line, and that file SHALL set no `thresholds.break`, so StrykerJS's exit status never depends on a score. It SHALL count a mutant StrykerJS reports `NoCoverage`, one the block's tests never run, as neither caught nor survived, unless every mutant StrykerJS reports for the block is `NoCoverage` and the block holds no `reads` line, when it SHALL count them all as survived, as for an empty test. It SHALL write `<id> killed` when the tests failed for a mutant, `<id> survived <n>` when `<n>` mutants built and the tests passed for every one, and `<id> not-judged` when no mutant built, when the block's tests run lines of its files but none a mutant changes and no mutant of its test files, when the block holds a `reads` line and its tests run no line of its files that has a mutant and no mutant of its test files, or StrykerJS reports every mutant `NoCoverage`, when the block's tests are all Playwright specs, or when its `code` lines name neither kind of file, a vitest test file counting as neither. After the blocks it SHALL run `.github/scripts/coverage.sh` and, when `CLM_LAND_DIR` is set, copy `target/lcov.info` to `$CLM_LAND_DIR/lcov.info`. Before the first block that needs a tool, it SHALL judge a fixture it writes itself with that tool, a function with one test that fails when the function is changed and one that passes whatever it returns: for `cargo-mutants` a crate in a temporary directory that the script makes a git repository with its files committed and from which it makes a tree the way it makes the checkout's, and whose tests pass only inside a git repository whose top is the crate and whose blind test also fails when git reports a tracked file changed from `HEAD` (`git status --porcelain --untracked-files=no`), so that a tool that leaves `.git` out of its copy, or a copy in which git sees the mutated file, fails the fixture, and which holds a test file under its `tests/` with a function that is no test and two tests of it, one that fails when that function is changed and one that passes whatever it returns, every mutant the tool lists for it tried as a block's test file's are, with no run to find the lines, and whose two tests also pass only inside a git repository whose top is the copy and, the blind one, fail when git reports a tracked file changed from `HEAD`, so that a copy without `.git` or one in which git sees the mutant, or a `cargo mutants --Zmutate-file` whose listing the script misreads, or a nextest whose exit status means something else, fails the fixture too, once, and for StrykerJS a source file and a test file inside the npm directory of the block, whose test that fails when the function is changed also reads a file of the repository outside that directory by a path relative to its own file, so that a sandbox from which the repository's files can't be reached fails the fixture, and which also fails when it finds beside its own file a directory that the script makes there and that a `.gitignore` the script writes beside the fixture's files names, so that a StrykerJS that copies what git ignores fails the fixture, once for each such directory before its first block, run with that directory's installed vitest and TypeScript, both removed afterwards; it SHALL go on only when the tool reports the first test's mutants caught and the second's missed, for the crate's function and for its test file's alike. It SHALL exit non-zero, naming the tool and the block or the fixture on stderr and writing no verdict for that block, when a tool can't be installed, when a tool's verdict on the fixture is wrong, when a tool fails for a reason other than its mutants' outcomes (`cargo mutants` exiting with a status other than 0, 2 or 3, as when the tests fail in the unmutated tree or a filterset doesn't parse; StrykerJS exiting non-zero, writing no report or running no test; `cargo mutants --Zmutate-file` exiting non-zero or listing what can't be read; `cargo nextest run` exiting for a test file's mutant with a status other than 0, 100 or 101), when the run that finds the lines a block's tests run, or a run of every test of one of its packages, fails or writes no report, or when `coverage.sh` fails; and 0 otherwise, whatever the verdicts. When a block fails so, it SHALL start no further block, let the blocks already running finish and write their lines, run no `coverage.sh`, and then exit non-zero. So a tool that doesn't work records no verdict for the block it failed in, and that block and every block not started stay due (`verify/mutation-pass`). No command `ci-check.sh` runs, nor any workflow step, SHALL be it. In the scenarios below `cargo`, `rustup` and `npx` are fakes on `PATH`, as in `coverage-script`'s, that install and mutate nothing; the fake `cargo llvm-cov` reports every line of the block's files run, and of the files of its `test` lines only when given `--no-default-ignore-filename-regex`, the fake `cargo mutants --Zmutate-file` lists no mutant of the repository's files and one of the fixture's test file, and the fake `cargo nextest run` exits 100 for the fixture's test that notices and 0 for every other run, unless a scenario says otherwise.

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

#### Scenario: StrykerJS mutates no test file
- GIVEN a plan with one block for a requirement of `plugin-panel/`, whose `code` lines name `plugin-panel/src/a.ts` and `plugin-panel/test/a.test.ts`, with a `diff` that changes both and one vitest `test` line naming `plugin-panel/test/a.test.ts`, and a fake `npx` whose `stryker run` records the file it is given and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN every range the recorded configuration's `mutate` names is of `src/a.ts`, and the result file holds `<id> killed`

#### Scenario: the fixture first
- GIVEN a plan with one Rust block, and a fake `cargo` that records each call
- WHEN `mutate.sh` runs
- THEN the first `cargo mutants` call ran in a crate outside the repository, before any call naming the block's files, and no file of the repository changed

#### Scenario: a tool that judges the fixture wrongly
- GIVEN the same with a fake `cargo` whose `mutants` reports every mutant caught, the fixture's blind test included
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming `cargo mutants` and the fixture, no call named the block's files, and the result file is empty

#### Scenario: the fixture's test file judged wrongly
- GIVEN a plan with one Rust block, and a fake `cargo` whose `mutants` judges the fixture's crate rightly, whose `mutants --Zmutate-file` lists one mutant of the fixture's test file, and whose `nextest run` exits 100 for each of the fixture's tests, the blind one included
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming the fixture, no call named the block's files, and the result file is empty

#### Scenario: Playwright only
- GIVEN a block whose `test` lines all name files under `web/e2e/`
- WHEN it runs
- THEN the result file holds `<id> not-judged`, and neither tool, nor its fixture, ran

#### Scenario: not in the land check
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, a stand-in `mutate.sh` beside it that records each run, and every test selected; and the two workflow files
- WHEN the script runs and the workflows are parsed
- THEN no recorded call runs `cargo mutants`, `stryker` or the stand-in `mutate.sh`, and no workflow step runs any of them

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

#### Scenario: tests that read the repository's files and run none of the code
- GIVEN the plan of the scenario *tests that run none of the code*, with a line `reads specs/**` in its block, and a fake `llvm-cov` that reports no line of the block's file run
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> not-judged`

#### Scenario: a rule in the test's own file
- GIVEN a plan with one Rust block with no `diff` whose one `test` line names a file under its crate's `tests/`, a fake `cargo` whose `mutants --list --json` lists one mutant of the block's code file on line 30, whose `llvm-cov` reports line 30 of the code file not run and lines 5 to 9 of the test file run, whose `mutants --Zmutate-file` lists for the test file two mutants, one spanning line 6 and one line 40, and whose `nextest run` records the directory it ran in and the test file there and exits 100
- WHEN `mutate.sh` runs
- THEN the `llvm-cov` call was given `--no-default-ignore-filename-regex`, no `cargo mutants` call tries a mutant of the code file, `nextest run` ran once for the block, in a directory that is neither the checkout nor the tree `cargo mutants` ran in, with the test file there changed as the mutant on line 6 says, the checkout's test file holds the same bytes as before the run, and the result file holds `<id> killed`

#### Scenario: a rule in the test's own file that its tests don't notice
- GIVEN the same with a fake `nextest run` that exits 0
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 1`

#### Scenario: a test file's mutant that doesn't build
- GIVEN the same with a fake `nextest run` that exits 101
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> not-judged`

#### Scenario: a rule in the test's own file of a block that reads files
- GIVEN the same as *a rule in the test's own file*, with a line `reads crates/**` in its block
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> killed`

#### Scenario: an empty test in a file with helpers
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports lines 2 and 3 of the test file run and no line of the code file
- WHEN `mutate.sh` runs
- THEN `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: a test that runs another component's code
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports line 4 of a Rust file of another crate run too
- WHEN `mutate.sh` runs
- THEN `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: code mutants first
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports line 30 of the code file run too, and a fake `mutants` that reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN the `cargo mutants` call that tries the block's mutants names the mutant on line 30, `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: nextest fails for a test file's mutant
- GIVEN the same as *a rule in the test's own file*, with a fake `nextest run` that exits 2
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo nextest` and the requirement, and the result file holds no line for it

#### Scenario: the run that finds the lines fails
- GIVEN the same with a fake `llvm-cov` that exits 1
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo llvm-cov` and the requirement, and the result file holds no line for it

#### Scenario: the further mutants of the requirement's own code first
- GIVEN a plan with one Rust block with no `diff`, one `test` line and `limit 1`, a fake `cargo` whose `mutants --list --json` lists two mutants of the block's file, one spanning line 3 alone and one line 30 alone, whose `llvm-cov` reports, for the call whose filterset names the block's test, lines 3 and 30 each run once and, for a call with no filterset, line 3 run 10 times and line 30 once, and whose `mutants` reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN one `llvm-cov` call had no filterset and named the block's test package, the `cargo mutants` call that tries the block's mutants names the mutant on line 30 and not the one on line 3, and the result file holds `<id> survived 1`

#### Scenario: no more further mutants than the limit
- GIVEN the same with `limit 2`
- WHEN `mutate.sh` runs
- THEN no `llvm-cov` call had no filterset, the `cargo mutants` call that tries the block's mutants names both mutants, and the result file holds `<id> survived 2`

#### Scenario: one run of a package's tests in a pass
- GIVEN a plan with two Rust blocks like the first of *the further mutants of the requirement's own code first*, whose tests are of the same package, and `CLM_MUTATE_JOBS` unset
- WHEN `mutate.sh` runs
- THEN exactly one `llvm-cov` call had no filterset, and the result file holds `survived 1` for each block

#### Scenario: the run of a package's tests fails
- GIVEN the plan and fake of *the further mutants of the requirement's own code first*, with an `llvm-cov` that exits 1 when called with no filterset
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

#### Scenario: StrykerJS finds no test running any mutant of a block that reads files
- GIVEN the same with a line `reads docs/**` in the block
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> not-judged`

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

#### Scenario: a TypeScript test that reads a file of the repository
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, and a fake `npx` whose `stryker run` records, for the `tempDirName` its configuration file names, each entry that directory holds and where it links, and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN that directory held a symbolic link for each entry of the repository's top but `.git` and `web`, to that entry; that directory is gone once the script has run, and the result file holds `<id> killed`

#### Scenario: the StrykerJS fixture reads a file of the repository
- GIVEN the same with a fake `npx` whose `stryker run` records the fixture's test file
- WHEN `mutate.sh` runs
- THEN the fixture's test that fails when its function is changed reads a file outside `web/` by a path relative to its own file

#### Scenario: StrykerJS copies nothing git ignores
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, a checkout whose `web/site/.gitignore` names `.test-dist/` and that has no `web/site/.test-dist/`, and a fake `npx` whose `stryker run` records the configuration file it is given and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN the configuration's `ignorePatterns`, read as `.gitignore` rules from `web/`, ignore `site/.test-dist/drift-src/docs` and not the block's `code` file or its test's file, and the result file holds `<id> killed`

#### Scenario: the StrykerJS fixture fails on what git ignores
- GIVEN the same with a fake `npx` whose `stryker run` records the fixture's test file and what the fixture's directory holds
- WHEN `mutate.sh` runs
- THEN the fixture's directory held a `.gitignore` naming a directory beside the fixture's files that the directory also held, and the fixture's test that fails when its function is changed also fails when it finds that directory beside its own file

#### Scenario: a bad environment
- GIVEN a plan with one Rust block, and `CLM_MUTATE` unset, then `CLM_MUTATE_RESULT` unset, then `CLM_SLOT=x`, the result file holding a line `x/old killed` where one is named
- WHEN `mutate.sh` runs, each time
- THEN each time it exits non-zero, stderr names the variable, holding `mutate.sh: CLM_SLOT is not a whole number: x` for the last, no fake ran, and the result file still holds `x/old killed`

#### Scenario: a result file from before
- GIVEN a plan with one Rust block, a fake `cargo` whose `mutants` reports one mutant caught, and a result file holding a line `x/old killed`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> killed` alone

#### Scenario: no nextest
- GIVEN a plan with one Rust block, and a fake `cargo` whose `nextest --version` exits 1
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo-nextest`, no `cargo mutants` call ran, and the result file is empty

#### Scenario: interrupted part-way
- GIVEN the plan and stand-in of *ended part-way*
- WHEN `mutate.sh` gets `SIGINT` once the first line is written, and in another run `SIGTERM`
- THEN each time it exits with status 143, the stand-in no longer runs, the result file holds the first block's line alone, and the temporary directory it made is gone

#### Serves
- architecture/proven-by-tests
- architecture/land-checks-stay-fast

#### Needs
- verify/mutation-pass
