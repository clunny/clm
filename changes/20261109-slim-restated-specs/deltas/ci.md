---
component: ci
---

## MODIFIED Requirements
### Requirement: nextest-runner <!-- base: dd9030d0c76fe032 -->
`.github/scripts/ci-check.sh` SHALL run the Rust tests with `cargo nextest run`, never with `cargo test` except for doctests: a full run SHALL run `cargo nextest run --workspace --no-tests=pass` and then `cargo test --doc --workspace`, and a selected run SHALL run the commands `selected-tests` states.

#### Scenario: full run
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, and `CLM_SELECTION` unset
- WHEN it runs
- THEN `cargo` ran `nextest run --workspace --no-tests=pass`, then `test --doc --workspace`, and no other `test`
