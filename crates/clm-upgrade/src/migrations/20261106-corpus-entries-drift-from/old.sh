#!/usr/bin/env bash
# Builds and tests this repository. `clm land` runs it before `clm check`; edit it to
# suit.
set -euo pipefail
cargo test
