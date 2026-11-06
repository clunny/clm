#!/usr/bin/env bash
# Builds and tests this repository. `clm land` runs it before `clm check`; edit it to
# suit.
set -euo pipefail
# Run from the repository's root, wherever this is run from.
cd "$(dirname "${BASH_SOURCE[0]}")/.."
