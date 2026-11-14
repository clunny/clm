---
component: ci
base: { contract: "2157b38bcbddfe69" }
---

## MODIFIED Contract

What this repository runs on GitHub Actions, and the check command `clm land` runs here (`land-locally`, `host-ci-for-releases`): a release workflow a human dispatches (bump the version, build prebuilt `clm` binaries, publish them as a GitHub Release and deploy the `web` app), a site workflow a human dispatches (build and deploy the public site, `site-worker-of-its-own`), and `.github/scripts/ci-check.sh`, the one command that runs every check of this repository, named in `clm.toml` as `[land] check`. Beside it, `.github/scripts/coverage.sh` produces `target/lcov.info`, the lcov file `clm health`'s `unexercised` signal reads (`lcov-coverage-signal`, `coverage-by-script`): a person or a session runs it by hand when they want that signal fresh, and `.github/scripts/mutate.sh`, the command of this repository's mutation pass (`mutate-script`), runs it at the end of each pass; no land, check or workflow runs either. No workflow runs on pull requests or pushes: checks run and branches land through `clm land`, on the machine that runs it.


## ADDED Requirements

### Requirement: mutate-script
`.github/scripts/mutate.sh` SHALL be a command for `[verify] mutate` (`verify/mutation-pass`): run from the repository root with `CLM_MUTATE` naming a plan and `CLM_MUTATE_RESULT` a file, it SHALL write one line for each `requirement <id>` block of the plan. For a block whose `code` lines name Rust files under `crates/`, it SHALL run `cargo mutants` at a version the script pins, installed with `cargo install --locked --version <x.y.z> cargo-mutants` when absent, with `--in-diff` on the block's `diff` file when it has one, and, when that finds no mutant or the tests pass for every one, on up to the block's `limit` further mutants of those files; the tests it runs for each mutant SHALL be the block's `test` lines alone, each mapped to its package, its test target and its name as `ci-check.sh` maps a selected file. For a block whose `code` lines name TypeScript files under `web/src/` or `plugin-panel/`, it SHALL run StrykerJS with its vitest runner at a pinned version, added to `node_modules/` alone with `npm install --no-save --ignore-scripts` so no `package.json` or lockfile changes, mutating the diff's lines first and then up to `limit` more, with the block's vitest `test` lines alone. It SHALL write `<id> killed` when the tests failed for a mutant, `<id> survived <n>` when `<n>` mutants built and the tests passed for every one, and `<id> not-judged` when no mutant built, when the block's tests are all Playwright specs, or when its `code` lines name neither kind of file. After the blocks it SHALL run `.github/scripts/coverage.sh` and, when `CLM_LAND_DIR` is set, copy `target/lcov.info` to `$CLM_LAND_DIR/lcov.info`. It SHALL exit non-zero when a tool can't be installed or `coverage.sh` fails, and 0 otherwise, whatever the verdicts. No workflow, and neither `ci-check.sh` nor `clm.toml`'s `[land] check`, SHALL run it. Its tests put fakes of `cargo` and `npx` on `PATH`, as `coverage-script`'s do, so they install and mutate nothing.

#### Scenario: a Rust requirement whose test notices
- GIVEN a plan with one block for a requirement of a crate, with a `diff` and one `test` line, and a fake `cargo` whose `mutants` reports one mutant caught
- WHEN `mutate.sh` runs
- THEN the fake was called with `--in-diff` and that test's package, target and name, and the result file holds `<id> killed`

#### Scenario: tests that notice nothing
- GIVEN the same with a fake that reports 3 mutants missed for the diff and 2 more missed for the files
- WHEN it runs
- THEN the result file holds `<id> survived 5`

#### Scenario: Playwright only
- GIVEN a block whose `test` lines all name files under `web/e2e/`
- WHEN it runs
- THEN the result file holds `<id> not-judged`

#### Scenario: not in the land check
- GIVEN `.github/scripts/ci-check.sh`, the workflows under `.github/workflows/` and `clm.toml`'s `[land]`
- WHEN they are read
- THEN none names `mutate.sh`, `cargo mutants` or `stryker`

#### Serves
- architecture/proven-by-tests
- architecture/land-checks-stay-fast

#### Needs
- verify/mutation-pass

## MODIFIED Requirements

### Requirement: coverage-script <!-- base: dd38930809b23429 -->
`.github/scripts/coverage.sh` SHALL write `target/lcov.info`, one lcov tracefile holding the workspace's coverage from `cargo llvm-cov --workspace --no-rustc-wrapper --lcov` and the `web/` and `plugin-panel/` vitest suites' from `@vitest/coverage-v8`, every `SF:` path naming its file relative to the repository root or absolutely under it. `--no-rustc-wrapper` puts the instrumentation in `RUSTFLAGS`, which the suite's nested `wasm32-unknown-unknown` builds remove, so those builds aren't instrumented. Before them it SHALL run `.github/scripts/detector-tools.sh`, `rustup component add llvm-tools-preview` and `cargo install --locked --version <x.y.z> cargo-llvm-cov`, pinned to the newest release at build time, and in each npm directory `npm ci --ignore-scripts` and then `npm install --no-save --ignore-scripts @vitest/coverage-v8@<x.y.z>` at the version `npx vitest --version` prints there, so no `package.json` or lockfile changes. It SHALL exit non-zero when any of these commands fails, and SHALL never pass `--ignore-run-fail`. No other file of this repository SHALL run it or `cargo llvm-cov` but `.github/scripts/mutate.sh` (`mutate-script`): not `ci-check.sh`, not the release workflow.

#### Scenario: one tracefile from three suites
- GIVEN the script with fakes on `PATH` for `cargo`, `rustup`, `npm` and `npx` that record their arguments, the `cargo` fake writing a record for `<tree>/crates/clm/src/main.rs` at the `--output-path` it is given, and the `npx` fake printing `vitest/5.0.5` for `--version` and writing `lcov.info` with a record for `src/app.ts` under the `--coverage.reportsDirectory` it is given
- WHEN it runs from the tree's root
- THEN `rustup` ran `component add llvm-tools-preview`, `cargo` ran `install --locked --version <x.y.z> cargo-llvm-cov` before `llvm-cov --workspace --no-rustc-wrapper --lcov`, `npm` ran `ci --ignore-scripts` and then `install --no-save --ignore-scripts @vitest/coverage-v8@5.0.5` in `web/` and in `plugin-panel/`, `npx` ran `vitest run` with `--coverage.enabled` in both, and `target/lcov.info` holds records for `<tree>/crates/clm/src/main.rs`, `web/src/app.ts` and `plugin-panel/src/app.ts`, in that order, and the script exits 0

#### Scenario: a failing suite fails the script
- GIVEN the same fakes, with an `npx` that exits 1 for `vitest run` in `plugin-panel/`
- WHEN it runs
- THEN it exits non-zero

#### Scenario: nothing else runs it
- GIVEN `.github/scripts/ci-check.sh`, `.github/scripts/detector-tools.sh` and the release workflow file
- WHEN their text is read
- THEN no line other than a comment names `coverage.sh` or `llvm-cov`

#### Scenario: pinned tools
- GIVEN the script's text
- WHEN it is read
- THEN a line runs `.github/scripts/detector-tools.sh` before the first `cargo` line, a line installs `cargo-llvm-cov` with `--locked` and an exact `--version`, the line that runs `cargo llvm-cov` passes `--no-rustc-wrapper` and no line passes `--ignore-run-fail`, and the line that installs `@vitest/coverage-v8` passes `--no-save`

#### Serves
- capability/know-health
