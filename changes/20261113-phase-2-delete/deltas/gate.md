---
component: gate
---

## MODIFIED Requirements

### Requirement: classify <!-- base: 17132c50496ce5cf -->
The gate SHALL classify a PR as `spec` if every changed file is a spec path, or if `rename/replay` accepts it (`replays(repo, <merge-base>, HEAD)` is true), `code` if no changed file is a spec path, and `mixed` otherwise. It SHALL block `mixed` PRs, telling the author to split them, and, when the branch is `impl/<component>`, telling its build that a build edits no spec path: it leaves a requirement that needs a spec edit unbuilt and adds a `question` backlog item whose `blocks` names it (`requirement-blocks`).

#### Scenario: mixed build branch
- GIVEN branch `impl/a` that changes `crates/a/lib.rs` and `specs/a/SPEC.md`
- WHEN the gate runs
- THEN it prints `mixed`, exits 1, and its line says the build leaves the requirement unbuilt and adds a `question` item

#### Scenario: mixed PR
- GIVEN a branch changing `specs/a/SPEC.md` and `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: rename branch
- GIVEN a branch from `main` holding only the committed result of `clm rename r`, which rewrote `specs/a/SPEC.md` and a marker in `crates/a/tests/t.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: rename branch with an extra code edit
- GIVEN that branch with one more commit changing `crates/a/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Needs
- rename/replay
