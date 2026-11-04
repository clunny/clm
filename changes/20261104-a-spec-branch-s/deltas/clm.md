---
component: clm
---

## ADDED Requirements

### Requirement: doc-readers-marked
Each test file under `crates/clm/tests/` that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or a file under `specs/`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`).

#### Scenario: the retired-terms test reads the specs
- GIVEN this repository and a change that touches only `specs/SPEC.md`
- WHEN `clm affected --readers` selects for that change
- THEN it selects `crates/clm/tests/retired_terms.rs`

#### Scenario: the root documents' readers
- GIVEN this repository and a change that touches only `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md`
- WHEN `clm affected --readers` selects for that change
- THEN it selects `crates/clm/tests/retired_terms.rs` and `crates/clm/tests/readme.rs`

#### Needs
- check.affected/affected-readers
