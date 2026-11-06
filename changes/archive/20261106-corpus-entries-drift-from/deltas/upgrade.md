---
component: upgrade
---

## ADDED Requirements

### Requirement: migration-check-script-from-root
An `advised` note migration SHALL bring a `scripts/clm-check.sh` that `clm init` wrote before its script ran from the repository's root, as `v0.14.0` and `v0.15.0` wrote it, to the text it writes today. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist or holds the line `cd "$(dirname "${BASH_SOURCE[0]}")/.."`. Its `old` SHALL be the script those releases' `clm init --lang rust` wrote, a bare `cargo test` after `set -euo pipefail`, and its `new` the script the current `clm init --lang rust` writes, so an unedited script is rewritten and any other is left to the note. Its note SHALL say what changed for every language: the script changes to the repository's root first, and runs each language's build and tests only when that language's manifest exists.

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an edited one
- GIVEN that script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID
