---
title: "This repository's lcov file comes from a script run by hand, outside the land's check"
date: 2026-10-28
status: accepted
---

`lcov-coverage-signal` leaves producing the lcov file to the repository. Here it is `.github/scripts/coverage.sh`, which runs the workspace's tests under `cargo llvm-cov`, the `web/` and `plugin-panel/` vitest suites under `@vitest/coverage-v8`, and concatenates the three tracefiles into `target/lcov.info`, the path `clm.toml` names under `[health] lcov`.

Nothing in `clm land`, `ci-check.sh` or the release workflow runs it. An instrumented run rebuilds every crate in a target directory of its own and runs the whole suite, which would double what a land costs for a signal that is advisory and never fails anything (`health-advises`). A person or a session runs it before `clm health` when the `unexercised` signal should be fresh; a stale or missing file costs one signal and nothing else.

The script installs its tools the way `detector-tools.sh` installs the detectors' (`ci/detector-tools`): `cargo-llvm-cov` pinned by `--locked --version`, the `llvm-tools-preview` component with `rustup component add`, and `@vitest/coverage-v8` with `npm install --no-save` at the version of the `vitest` each directory's `npm ci` installed, so `node_modules/` holds it for the run and no `package.json` or lockfile changes.

Rejected:
- **A step of `ci-check.sh` behind a flag.** The script's tests check the exact commands of a run; a flag doubles their matrix for a step no land takes, and coverage isn't a check.
- **A devDependency in `web/` and `plugin-panel/`.** Each is another node's file, so each would need a delta and a build (`dependency-work-is-a-requirement`), for a package only this script uses.
- **`c8` around vitest.** Vitest transforms TypeScript in memory; `c8` maps V8's offsets onto the files on disk. `@vitest/coverage-v8` is the provider vitest ships for its own transforms.
- **Listing `llvm-tools-preview` in `rust-toolchain.toml`.** Every `rustup show` would then install a component only this script uses.
