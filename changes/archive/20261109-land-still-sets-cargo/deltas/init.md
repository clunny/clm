---
component: init
---

## ADDED Requirements
### Requirement: starter-check-land-dir
The `scripts/clm-check.sh` that `clm init` writes for `--lang rust` SHALL, in the lines it runs when `Cargo.toml` exists and before `cargo test`, export `CARGO_TARGET_DIR` set to the value of `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` when `CLM_LAND_DIR` is set and non-empty, and leave both as its caller has them otherwise. So a land's cargo build stays warm in the place `clm land` hands it (`land/land-dir-env`), from land to land, and a build's own check, which gets no `CLM_LAND_DIR`, compiles into its worktree's `target/`. A repository with no `Cargo.toml` SHALL still pass the script, as `init-without-manifest` says. Since an older project holds the script without these lines, `clm init`'s change to them ships with `upgrade/migration-starter-land-dir`, in one build (`template-change-ships-its-migration`).

#### Scenario: a land hands the place
- GIVEN a repository `clm init --lang rust` set up, holding a `Cargo.toml`, and on `PATH` a `cargo` that records `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` and exits 0
- WHEN `scripts/clm-check.sh` runs with `CLM_LAND_DIR=/tmp/x/land` and both cargo variables unset
- THEN the stub saw `CARGO_TARGET_DIR=/tmp/x/land` and `CARGO_INCREMENTAL=0`

#### Scenario: no place handed
- GIVEN the same repository and stub
- WHEN `scripts/clm-check.sh` runs with `CLM_LAND_DIR` unset, `CARGO_TARGET_DIR=/tmp/y` and `CARGO_INCREMENTAL` unset
- THEN the stub saw `CARGO_TARGET_DIR=/tmp/y` and `CARGO_INCREMENTAL` unset

#### Serves
- capability/adopt-clm
- capability/work-concurrently
