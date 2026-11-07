---
status: applied
---

## Intent

Make `clm land`'s check of this repository faster without checking less: run the Rust tests under `cargo-nextest`, which runs a package's test binaries at once instead of one after another, and have the check script print how long each phase took, so the next speed-up is chosen from numbers. Lands are serial behind the check lock, so every second a check spends is spent again by every land queued behind it.

## Problem

A full land check measured on 2026-11-07 took 388 seconds with a warm build: `cargo test --all` 179 s (15 s compiling, about 160 s running 367 test binaries one at a time on a 24-core machine), the `web/`, `web/site/` and `capabilities/web/` suites 167 s together, the wasm builds 28 s, clippy 7 s, the rest under 10 s. The script prints no times, so this took a traced run to find out. Within the tests, one binary, `crates/clm/tests/guard.rs`, takes 30 s by waiting out the guard's real budgets; no other test is slow in the sense `verify` is for.

## Approaches

- **Run the tests in parallel with `cargo-nextest`, and time the phases (chosen).** Loses no check; the widely used runner does the parallelism (`reuse-first`). Doctests, which `nextest` doesn't run, follow in a full run with `cargo test --doc`.
- **Move some Rust tests after the land, into `[verify]`.** Rejected: none is slow in itself; the time is the serial run and three tests that wait out real timeouts. Moving them would give up gating on rules that each run in well under a second.
- **Skip the wasm builds and clippy when the selection can't affect them.** Deferred: about 35 s, a smaller win than the runner and fiddlier to get right.
- **Shorten the guard's timeout tests in this change.** Split off to `backlog/guard-timeout-tests-wait-out-real-budgets`: `20261107-the-push-guard-blocks` is rewriting the same requirement.

The browser and site suites move to `[verify]` once `20261107-slow-tests-after-land` is built; that needs no change of its own beyond this repository's `clm.toml`.

## Outcome

- `ci/nextest-runner`: `ci-check.sh` runs `cargo nextest run --workspace --no-tests=pass` and `cargo test --doc --workspace` for a full run, and `cargo nextest run --no-tests=pass -p <package> <targets>` per selected package (`ci/selected-tests`, modified).
- `ci/test-tools`: `.github/scripts/test-tools.sh` installs a pinned `cargo-nextest`; the release keeps `cargo test --all` as its backstop and doesn't install it.
- `ci/phase-times`: each phase's time on stderr, and a summary on exit, passing or failing.

## Constraints

- Every test must pass while others run beside it, as `tests-run-concurrently` asks. The `ci` build changes no other component's test (`path-ownership`): a test that fails under `nextest` becomes a `kind: bug` item on its component blocking `ci/nextest-runner`, and the script moves once those are fixed.
- On a failure the script prints one line naming the failed phase rather than the whole summary, so the 40 lines a refused land shows stay the failure's (`land/land-check-log`).
- The phase lines go to stderr, so the output's tail on a failed land stays the failure's.
