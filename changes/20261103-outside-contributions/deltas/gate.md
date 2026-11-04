---
component: gate
---

## ADDED Requirements

### Requirement: files-outside-spec-paths
`clm-gate` SHALL offer a library call returning the files a revision changes against a base, from the merge base as `classify` reads them, that aren't spec paths, sorted, and returning none when `rename/replay` accepts the revision. It SHALL read the revision only through git, never the working tree or the index, as `gate/evaluate-at` does.

#### Scenario: a code file among spec files
- GIVEN a revision changing `specs/a/SPEC.md` and `crates/x/src/lib.rs` against its base
- WHEN the call runs with something else checked out
- THEN it returns `crates/x/src/lib.rs` alone

#### Scenario: only a new backlog item
- GIVEN a revision that only adds `backlog/x.md`, for which `clm gate` prints `code`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: a rename branch
- GIVEN the committed result of `clm rename r`, which rewrote a marker in `crates/a/tests/t.rs`
- WHEN the call runs
- THEN it returns nothing

#### Serves
- capability/contribute-from-outside
