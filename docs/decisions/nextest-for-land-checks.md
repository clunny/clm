---
title: "This repository's land check runs its Rust tests under cargo-nextest, and times its phases"
date: 2026-11-07
status: accepted
---

A full land check took 388 s in a traced run on 2026-11-07, 179 s of it `cargo test --all`, which spent about 160 s running 367 test binaries one after another on a 24-core machine. No Rust test was slow in itself apart from three guard tests that wait out real 20- and 30-second budgets. So the tests stay in the land check and run in parallel under `cargo-nextest` (`reuse-first`), with doctests, which it doesn't run, after it in a full run; and the script prints each phase's time, so the next cut is chosen from numbers rather than a guess.

- `nextest` is installed by `.github/scripts/test-tools.sh`, not `detector-tools.sh`: the release's `prepare` job runs the latter and keeps `cargo test --all` as the full backstop, so it shouldn't build `nextest`.
- Selected runs keep one run per package, as `cargo test` had, so the build stays the same and a package's binaries run at once; one run across packages with a filter expression would build every test target of each package, more than a `--test` selection builds.
- Phase times go to stderr, keeping a failed land's output tail the failure's.

Rejected:
- **Moving Rust tests to `[verify]`.** Gives up gating on fast rules for time that was the runner's, not the tests'.
- **A test-only variable that shortens the guard's budgets.** A setting that changes a safety hook's budget in production; the library-parameter seam in `backlog/guard-timeout-tests-wait-out-real-budgets` proves the same rules.
