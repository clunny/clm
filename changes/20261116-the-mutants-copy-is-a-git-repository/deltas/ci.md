---
component: ci
---

## MODIFIED Requirements

### Requirement: mutate-script <!-- base: 85b28dfb326e2f0b -->
`.github/scripts/mutate.sh` SHALL be a command for `[verify] mutate` (`verify/mutation-pass`): run from the repository root with `CLM_MUTATE` naming a plan and `CLM_MUTATE_RESULT` a file, it SHALL write one line for each `requirement <id>` block of the plan. For a block whose `code` lines name Rust files under `crates/`, it SHALL run `cargo mutants` at a version the script pins, installed with `cargo install --locked --version <x.y.z> cargo-mutants` when absent, with `--in-diff` on the block's `diff` file when it has one, and, when that finds no mutant or the tests pass for every one, on up to the block's `limit` further mutants of those files; it SHALL have `cargo mutants` copy the checkout's `.git` into the copy of the tree it tests (`--copy-vcs true`), so that a test that asks git about this repository passes there as it does in the checkout; the tests it runs for each mutant SHALL be the block's `test` lines alone, each given to nextest as a filterset naming its package, its test target by nextest's binary ID, `binary_id(=<package>::<target>)`, where `ci-check.sh` passes `--test <target>` for a selected file, and its name. For a block whose `code` lines name TypeScript files under `web/src/` or `plugin-panel/`, it SHALL run StrykerJS with its vitest runner at a pinned version, added to `node_modules/` alone with `npm install --no-save --ignore-scripts` so no `package.json` or lockfile changes, mutating the diff's lines first and then up to `limit` more, with the block's vitest `test` lines alone; it SHALL give StrykerJS its test runner, the vitest configuration that names those tests, its reporter and its `mutate` ranges in a configuration file it writes under that directory's `node_modules/`, since StrykerJS takes a runner's options from no command line, and that file SHALL set no `thresholds.break`, so StrykerJS's exit status never depends on a score. It SHALL write `<id> killed` when the tests failed for a mutant, `<id> survived <n>` when `<n>` mutants built and the tests passed for every one, and `<id> not-judged` when no mutant built, when the block's tests are all Playwright specs, or when its `code` lines name neither kind of file. After the blocks it SHALL run `.github/scripts/coverage.sh` and, when `CLM_LAND_DIR` is set, copy `target/lcov.info` to `$CLM_LAND_DIR/lcov.info`. Before the first block that needs a tool, it SHALL judge a fixture it writes itself with that tool, a function with one test that fails when the function is changed and one that passes whatever it returns: for `cargo-mutants` a crate in a temporary directory, once, and for StrykerJS a source file and a test file inside the npm directory of the block, once for each such directory before its first block, run with that directory's installed vitest and TypeScript, both removed afterwards; it SHALL go on only when the tool reports the first test's mutants caught and the second's missed. It SHALL exit non-zero, naming the tool and the block or the fixture on stderr and writing no verdict for that block, when a tool can't be installed, when a tool's verdict on the fixture is wrong, when a tool fails for a reason other than its mutants' outcomes (`cargo mutants` exiting with a status other than 0, 2 or 3, as when the tests fail in the unmutated tree or a filterset doesn't parse; StrykerJS exiting non-zero, writing no report or running no test), or when `coverage.sh` fails; and 0 otherwise, whatever the verdicts. So a tool that doesn't work leaves nothing recorded, and the requirements of the plan stay due (`verify/mutation-pass`). No workflow, and neither `ci-check.sh` nor `clm.toml`'s `[land] check`, SHALL run it. Its tests put fakes of `cargo` and `npx` on `PATH`, as `coverage-script`'s do, so they install and mutate nothing.

#### Scenario: a Rust requirement whose test notices
- GIVEN a plan with one block for a requirement of a crate, with a `diff` and one `test` line, and a fake `cargo` whose `mutants` reports one mutant caught
- WHEN `mutate.sh` runs
- THEN the fake was called with `--in-diff` and a filterset holding `package(=<package>)`, `binary_id(=<package>::<target>)` and the test's name, no `binary(` among its words, and the result file holds `<id> killed`

#### Scenario: tests that notice nothing
- GIVEN the same with a fake that reports 3 mutants missed for the diff and 2 more missed for the files
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

#### Serves
- architecture/proven-by-tests
- architecture/land-checks-stay-fast

#### Needs
- verify/mutation-pass
