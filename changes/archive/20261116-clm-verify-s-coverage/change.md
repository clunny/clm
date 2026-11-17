---
status: done
human_burdens:
  - "Each release's prepare job builds cargo-nextest: a few more minutes per release"
resolves: [clm-verify-s-coverage-step-hangs-forever-on-tests-that-change]
---

## Intent

A verify must finish or fail in bounded time, not hang on a test for ever. Every run of this repository's Rust tests goes through `cargo nextest`, one process per test, which is what the tests are written for: the coverage pass through `cargo llvm-cov nextest` and the release's `prepare` job too, which ran them with `cargo test`, and a session, as `AGENTS.md` now says. And nextest ends any test still running after 15 minutes and counts it failed, so a hang fails the run and names the test. Decision: `one-runner-for-the-rust-tests`. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

On 2026-11-16 a build loop's `clm verify --due` ran 4 hours 40 minutes. Its mutation pass had reached `coverage.sh`, whose `cargo llvm-cov` runs the tests through `cargo test`, each test binary's tests on threads of one process. One binary spun for 100 minutes with no output, about 20 processes at ~68% CPU each, until the session killed the tree by hand; verify recorded nothing.

The cause is a mismatch of runners, not of one test. `commands-live-in-their-crates` lets a test set its own process's `PATH`, `TMPDIR` or `GIT_` variables for the length of a call, because the tests run one per process (`ci/nextest-runner`). The helpers that do so, in the test files of at least `orchestrator.checkpoint`, `land`, `ledger`, `host`, `init`, `orchestrator.triage` and `check.unreferenced`, say so in their `SAFETY` comments. That holds under nextest, which `ci-check.sh` uses. It doesn't under `cargo test`: there the tests stacked each other's fake-`git` directories on the one `PATH`, and each fake, finding the real `git` by stripping only its own directory, ran another test's fake, which ran it back. Three places still run the suite under `cargo test`: `coverage.sh`, the release's `release-prepare.sh` (`cargo test --all`, kept as a "backstop" by `ci/test-tools`) and `AGENTS.md`'s instruction to sessions.

And nothing bounds a test's time: not nextest's configuration (there is none), not the coverage run, not the mutation command. So a hang is never reported, and the loop offers to run the same verify again, which would hang again.

## Approaches

- **One runner everywhere, and a bound on each test in nextest's configuration.** Taken. It makes the assumption the tests already rely on hold wherever they run, changes no test, and the bound lives in `.config/nextest.toml`, which nextest reads from the workspace root in every run, the copies `cargo mutants` tests in included, and names the test that hung.
- **Make each helper safe under `cargo test`**, by a lock shared across a test binary or by passing the environment to the command under test. Not taken: it edits the tests of seven or more components for a runner no check needs, `commands-live-in-their-crates` already rejected threading an environment through the calls, and a lock serialises the tests that touch it.
- **`cargo test -- --test-threads=1`** for coverage and the release. Not taken: slower, and a hang is still unbounded.
- **A limit on the whole coverage, check or mutation command.** Not taken, as for npm installs: it must be loose enough for the slowest whole run, so a hang still costs hours, and it names no test.

## Outcome

`ci` (`deltas/ci.md`):
- `ci/tests-time-bounded` (new): `.config/nextest.toml` sets `[profile.default] slow-timeout = { period = "60s", terminate-after = 15 }`; no script passes another profile or configuration file. `.config/nextest.toml` joins `ci`'s `paths`.
- `ci/coverage-script` (modified): `cargo llvm-cov nextest --workspace --no-rustc-wrapper --lcov`, after `.github/scripts/test-tools.sh`; a failing Rust run fails the script.
- `ci/nextest-runner` (modified): every script and workflow runs the Rust tests under nextest and no test but a doctest under `cargo test`; `release-prepare.sh` runs `cargo nextest run --workspace --no-tests=pass` then `cargo test --doc --workspace`.
- `ci/test-tools` (modified): `coverage.sh` and the release's `prepare` job run `test-tools.sh` too; the "not in the release" scenario is replaced by its opposite.
- The Design's coverage, release and running-the-tests paragraphs say the same, with the reason.

Hand edits: `AGENTS.md` tells a session to run `cargo nextest run` and `cargo test --doc`, never plain `cargo test`. `docs/decisions/one-runner-for-the-rust-tests.md` records the choice.

Backlog: `backlog/a-verify-can-hang-on-a-test-until-the-plan-s-freeze-ends.md`, a `question` blocking the four requirements until the human says whether to build them as an exception to the freeze.

## What it removes

- `ci/test-tools`'s scenario "not in the release" (the release workflow and `release-prepare.sh` name neither `test-tools.sh` nor `nextest`) and its sentence "No workflow and no other script SHALL run it", with the release's `cargo test --all` run. They are replaced by a scenario that the release runs `test-tools.sh` and the tests under nextest. `removal-needs-a-yes`: the human's approval of this change is that yes.

## Constraints and known edge cases

- The release's `prepare` job builds `cargo-nextest` with `cargo install`, a few minutes on its hosted runner.
- 15 minutes must lie past the slowest test, instrumented under coverage and on a loaded machine. The build checks the slowest test's time in a coverage or full run; a test that needs longer is made faster, not the bound raised.
- `cargo llvm-cov nextest` runs no doctest, as plain `cargo llvm-cov` didn't either; doctests stay unbounded under `cargo test --doc`.
- The fake `git` helpers keep stripping only their own directory: under one process per test there is only one.
- A person who runs plain `cargo test` by hand can still meet the hang; `AGENTS.md` says not to.
- `ci/tests-time-bounded` is proven at the land by reading the file only; no land-check test runs the real `nextest` against a hanging test (`land-checks-stay-fast`).
- `ci/npm-installs-bounded`, also unbuilt and blocked by the plan's freeze, edits the same `coverage.sh` lines and tests; whichever `ci` build comes second rebases on the first.
- The builds of these requirements also fix the comments they make stale: the headers of `test-tools.sh` and `detector-tools.sh`, the comment on `coverage.sh`'s `cargo llvm-cov` line and `release.yml`'s prepare-step comment; the test `the_release_builds_no_nextest` in `crates/clm/tests/ci_check.rs` is replaced, and `release_workflow.rs`'s fake `cargo` records `nextest run` and `test --doc` instead of passing them to the real cargo.

## Open questions

None.
