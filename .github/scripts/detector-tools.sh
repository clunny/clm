#!/usr/bin/env bash
# Installs the tools behind `clm.toml`'s detectors (specs/ci/SPEC.md, ci/detector-tools).
# `ci-check.sh` and the release workflow's `prepare` job run it before `cargo test`, since
# `this_repository_is_clean` runs `clm check`. A detector added to `clm.toml` adds its tool here,
# except one whose command is `clm`: it runs the clone's own `clm` (scripts/dev-install.sh),
# or the binary under test, or `sh` on a file of the repository (it needs only `sh` and `grep`).
set -euo pipefail

cargo install --locked --version 0.9.2 cargo-machete
