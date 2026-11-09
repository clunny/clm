---
component: upgrade
---

## ADDED Requirements
### Requirement: migration-starter-land-dir
An `advised` note migration, ID `20261109-land-still-sets-cargo`, SHALL bring a `scripts/clm-check.sh` that `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0` to the text it writes since `init/starter-check-land-dir`, since `clm land` no longer sets cargo's variables for a project's check (`land/shared-land-target`). Its `file` SHALL be `scripts/clm-check.sh`, its `old` the script those releases' `clm init --lang rust` wrote (one text, the same in each), and its `new` the script the current `clm init --lang rust` writes, so an unedited script is rewritten and any other is left to the note. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist, holds `CLM_LAND_DIR`, or doesn't hold `cargo`, and false otherwise. Its note's first line SHALL read `Lands no longer set CARGO_TARGET_DIR: point cargo at CLM_LAND_DIR in your check`. The note SHALL say:
- that `clm land` no longer sets `CARGO_TARGET_DIR` or `CARGO_INCREMENTAL` for `[land] check`, and hands it `CLM_LAND_DIR` instead, the clone's build place that lasts from land to land, while `clm land --check-only` hands none, so a build's check compiles in its own worktree;
- that a check script that runs cargo should, before its first cargo command, export `CARGO_TARGET_DIR` from `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` when `CLM_LAND_DIR` is set and non-empty, giving the lines `clm init` writes for it;
- that without them a land's cargo build goes to the land tree's own `target/`, which the land cleans before every check, so every land compiles from scratch;
- that a project whose check keeps its build output elsewhere on purpose declines the note with `clm upgrade --decline 20261109-land-still-sets-cargo`.

#### Scenario: an unedited 0.23 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.23.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, after `migration-check-script-from-root`'s rewrite and this one's

#### Scenario: an edited one
- GIVEN the `v0.23.0` script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID

#### Scenario: a script that runs no cargo
- GIVEN a `scripts/clm-check.sh` that `clm init --lang typescript` wrote
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261109-land-still-sets-cargo` runs on a project it is pending in
- THEN the note names `CLM_LAND_DIR`, `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `--decline`

#### Needs
- clm/upgrade-command
- init/starter-check-land-dir

## MODIFIED Requirements
### Requirement: migration-check-script-from-root <!-- base: 0f7f933f5988908d -->
An `advised` note migration SHALL bring a `scripts/clm-check.sh` that `clm init` wrote before its script ran from the repository's root, as `v0.14.0` and `v0.15.0` wrote it, to the text `v0.16.0` wrote. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist or holds the line `cd "$(dirname "${BASH_SOURCE[0]}")/.."`. Its `old` SHALL be the script those releases' `clm init --lang rust` wrote, a bare `cargo test` after `set -euo pipefail`, and its `new` the script `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0`, which `migration-starter-land-dir` then brings to the text written today, so an unedited script is rewritten and any other is left to the note. Its note SHALL say what changed for every language: the script changes to the repository's root first, and runs each language's build and tests only when that language's manifest exists.

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an edited one
- GIVEN that script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID
