---
component: init
---

## ADDED Requirements
### Requirement: starter-land-dir
When `--lang rust` is given, the `scripts/clm-check.sh` that `clm init` writes SHALL, before its `cargo test` line, export `CARGO_TARGET_DIR` set to `CLM_LAND_DIR` when `CLM_LAND_DIR` is set and non-empty (`land/land-dir-env`), and leave it as the caller has it otherwise, with a comment saying that `clm land` hands the check a place of the clone's that lasts from land to land, so the build stays warm. A starter written for other languages alone SHALL not mention `CARGO_TARGET_DIR`.

#### Scenario: a Rust starter
- GIVEN a repository with a `Cargo.toml` and a fake `cargo` on `PATH` that records `CARGO_TARGET_DIR`
- WHEN `clm init --lang rust` runs and then `scripts/clm-check.sh` runs with `CLM_LAND_DIR=/tmp/x/land`
- THEN the fake recorded `/tmp/x/land`

#### Scenario: a Python starter
- GIVEN an empty repository
- WHEN `clm init --lang python` runs
- THEN `scripts/clm-check.sh` doesn't mention `CARGO_TARGET_DIR`
