#!/usr/bin/env bash
# Builds and tests this repository. `clm land` runs it before `clm check`; edit it to
# suit.
set -euo pipefail
# Run from the repository's root, wherever this is run from.
cd "$(dirname "${BASH_SOURCE[0]}")/.."
if [ -f Cargo.toml ]; then
    # A land hands its check the place its build lasts in; a build's own check gets none.
    if [ -n "${CLM_LAND_DIR:-}" ]; then
        export CARGO_TARGET_DIR="$CLM_LAND_DIR" CARGO_INCREMENTAL=0
    fi
    cargo test
fi
