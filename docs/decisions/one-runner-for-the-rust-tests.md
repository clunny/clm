---
title: "Every run of this repository's Rust tests goes through nextest, and nextest ends a test that hangs"
date: 2026-11-16
status: accepted
---

On 2026-11-16 a build loop's `clm verify --due` ran for 4 hours 40 minutes before a person asked why. Its mutation pass had reached `coverage.sh`, which ran the workspace's tests through `cargo llvm-cov`, that is `cargo test`: each test binary's tests on threads of one process. One binary never finished. Its tests put a fake `git` on their own process's `PATH` for the length of a call, which `commands-live-in-their-crates` takes as safe because the tests run one per process (`ci/nextest-runner`). Side by side in one process they stacked each other's directories on the one `PATH`, and each fake, finding the real `git` by stripping only its own directory from the front, ran another test's fake, which ran it back, for ever. About 20 processes spun for 100 minutes with no output, and nothing would have ended them: no bound lay on a test, on the coverage run or on the mutation command.

Such helpers sit in the test files of at least `orchestrator.checkpoint`, `land`, `ledger`, `host`, `init`, `orchestrator.triage` and `check.unreferenced`. The release's `prepare` job runs the same suite with `cargo test --all` and would meet the same hang. And `AGENTS.md` told a session to run `cargo test`.

**One runner.** Every run of the Rust tests other than the doctests goes through `cargo nextest`: `ci-check.sh`'s, as before; `coverage.sh`'s, through `cargo llvm-cov nextest`; `mutate.sh`'s for each mutant, as before; the release's `prepare` job's, which now installs `nextest` with `test-tools.sh`; and a session's, as `AGENTS.md` now says. A test of this repository may then rely on owning its process, as `commands-live-in-their-crates` decided, wherever it runs.

**A bound on each test.** `.config/nextest.toml` sets the default profile's `slow-timeout` to end a test after 15 minutes and count it failed (`ci/tests-time-bounded`). nextest reads the file from the workspace root wherever it runs, so the bound reaches every run above with nothing on a command line, and the run fails naming the test that hung. A test that needs longer is to be made faster.

Rejected:
- **Making each helper safe under `cargo test`**, with a shared lock per test binary or the environment passed to the command under test. It changes the test files of seven or more components for a runner no check uses, `commands-live-in-their-crates` rejected threading an environment through the calls, and a lock serialises every test that touches the environment.
- **`cargo test -- --test-threads=1`** for coverage and the release: one test at a time also gives a test its process to itself, but makes the instrumented run and the release's run far slower, and a hang stays unbounded.
- **A time limit on the whole check, coverage or mutation command**: it must be loose enough for the slowest whole run on a loaded machine, so a hang still costs hours, and it names no test (`npm-installs-bounded` rejected the same).
- **Keeping the release free of `nextest`**, as `nextest-for-land-checks` and `ci/test-tools` said, with `cargo test --all` as a backstop: the backstop would hang on the same tests, and the release's few minutes to build `nextest` are the cheaper cost.
