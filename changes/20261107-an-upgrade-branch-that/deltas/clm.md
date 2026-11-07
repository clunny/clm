---
component: clm
---

## MODIFIED Requirements

### Requirement: doc-readers-marked <!-- base: 54c98381a9f2cdc8 -->
Each test file under `crates/clm/tests/` that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md` or a file under `specs/`, or that reads the repository's own `clm.toml`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`) or counts as checked because only `clm.toml`'s comments or upgrade notes changed (`land/settings-unchanged-is-checked`).

#### Scenario: the retired-terms test reads the specs
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `specs/SPEC.md` as the only differing path
- THEN it selects `crates/clm/tests/retired_terms.rs`

#### Scenario: the root documents' readers
- GIVEN the same markers
- WHEN `readers` runs on them with `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md` as the differing paths
- THEN it selects `crates/clm/tests/retired_terms.rs` and `crates/clm/tests/readme.rs`

#### Scenario: the readers of clm.toml
- GIVEN the same markers
- WHEN `readers` runs on them with `clm.toml` as the only differing path
- THEN it selects `crates/clm/tests/cli.rs`, whose allow-list test reads `clm.toml`'s comments

#### Needs
- check.affected/affected-readers
