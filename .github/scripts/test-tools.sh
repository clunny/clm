#!/usr/bin/env bash
# Installs the test runner (specs/ci/SPEC.md, ci/test-tools). Only `ci-check.sh` runs it, after
# `detector-tools.sh` and before its first `cargo` line; the release's `prepare` job keeps
# `cargo test --all` and doesn't build `cargo-nextest`.
set -euo pipefail

cargo install --locked --version 0.9.151 cargo-nextest
