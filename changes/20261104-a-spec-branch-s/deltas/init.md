---
component: init
---

## ADDED Requirements

### Requirement: doc-readers-marked
Each test file under `crates/clm-init/tests/` that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`).

#### Scenario: CLM's own ARCHITECTURE.md
- GIVEN this repository and a change that touches only `ARCHITECTURE.md`
- WHEN `clm affected --readers` selects for that change
- THEN it selects `crates/clm-init/tests/says_component.rs`

#### Needs
- check.affected/affected-readers
