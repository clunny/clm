#!/usr/bin/env bash
# Installs the test runner (specs/ci/SPEC.md, ci/test-tools). `ci-check.sh` and `coverage.sh`
# run it after `detector-tools.sh` and before their first `cargo` line, and the release's
# `prepare` job in a step of its own before `release-prepare.sh`; nothing else does.
set -euo pipefail

cargo install --locked --version 0.9.151 cargo-nextest
