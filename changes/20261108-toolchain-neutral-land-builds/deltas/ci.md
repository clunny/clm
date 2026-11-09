---
component: ci
---

## ADDED Requirements
### Requirement: land-dir
When `CLM_LAND_DIR` is set and non-empty, `.github/scripts/ci-check.sh` SHALL export `CARGO_TARGET_DIR` set to it and `CARGO_INCREMENTAL=0` before its first cargo command, so this repository's lands keep their cargo output in the place `clm land` hands them (`land/land-dir-env`) once `clm land` stops setting the cargo variables itself; when it is unset or empty, the script SHALL leave both variables as its caller has them.

#### Scenario: a land's place
- GIVEN the script's test tree with fakes for `cargo` that record `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL`, and `CLM_LAND_DIR=/tmp/x/land` with both cargo variables unset
- WHEN the script runs
- THEN every recorded `cargo` call saw `CARGO_TARGET_DIR=/tmp/x/land` and `CARGO_INCREMENTAL=0`

#### Scenario: no place
- GIVEN the same with `CLM_LAND_DIR` unset and `CARGO_TARGET_DIR=/tmp/y`
- WHEN the script runs
- THEN every recorded `cargo` call saw `CARGO_TARGET_DIR=/tmp/y`

#### Needs
- land/land-dir-env
