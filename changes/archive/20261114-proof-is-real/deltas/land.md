---
component: land
---

## ADDED Requirements

### Requirement: build-holds-own-files
`clm land <branch>`, for a branch named `impl/<component>`, SHALL exit 1 when `gate/own-files`' `foreign_files` lists any file for its head against the fetched base and that component, printing for each `<file>: land/build-holds-own-files: owned by `<owner>`; impl/<component> changes its own files, the wiring, a test that cites it and a capability test it serves. Build this in `<owner>`, or leave the requirement unbuilt and add a question item that blocks it`. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. `clm land --check-only`, in a checkout whose branch is `impl/<component>`, SHALL print the same lines for `HEAD` against the local `origin/main` and exit 1 before it runs the check command, so a build learns of it before its land. A branch with any other name, `impl/archive/<id>` included, and a detached `HEAD`, SHALL land or check as before.

#### Scenario: another component's source
- GIVEN a root with `wiring: []`, components `a` and `b`, and a branch `impl/a` that changes `crates/a/src/lib.rs` and `crates/b/src/lib.rs`, and a check command that records that it ran
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `crates/b/src/lib.rs` and `b` under `land/build-holds-own-files`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: the build's pre-check
- GIVEN the same branch checked out
- WHEN `clm land --check-only` runs
- THEN it prints the same line, exits 1, and the check command never ran

#### Scenario: its own files
- GIVEN the same root and a branch `impl/a` that changes only `crates/a/src/lib.rs`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a branch of another name
- GIVEN the same root and a branch `tidy-b` that changes `crates/a/src/lib.rs` and `crates/b/src/lib.rs`
- WHEN `clm land tidy-b` runs
- THEN this rule refuses nothing

#### Scenario: not switched on
- GIVEN a root with no `wiring`, and the first scenario's branch
- WHEN `clm land impl/a` runs
- THEN it lands

#### Serves
- architecture/path-ownership
- capability/land-safely

#### Needs
- gate/own-files
