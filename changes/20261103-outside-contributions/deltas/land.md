---
component: land
---

## ADDED Requirements

### Requirement: spec-branch-holds-spec-paths
`clm land <branch>`, for a branch named `spec/*`, SHALL exit 1 when `gate/files-outside-spec-paths` lists any file for its head against the fetched base, naming each file and saying a spec branch holds only spec paths. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. A branch with any other name SHALL land as before.

#### Scenario: code on a spec branch
- GIVEN a branch `spec/x` that changes only `crates/x/src/lib.rs`, whose gate prints `code`, and a check command that records that it ran
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `crates/x/src/lib.rs`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a suggestion on a spec branch
- GIVEN a branch `spec/x` that only adds `backlog/x.md`, whose gate prints `code`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: code on an impl branch
- GIVEN a branch `impl/x` that changes only `crates/x/src/lib.rs`
- WHEN `clm land impl/x` runs
- THEN this rule refuses nothing and it lands

#### Needs
- gate/files-outside-spec-paths

#### Serves
- capability/contribute-from-outside
- architecture/ai-writes-code
