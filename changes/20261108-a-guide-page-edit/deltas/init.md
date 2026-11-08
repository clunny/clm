---
component: init
---

## MODIFIED Requirements
### Requirement: doc-readers-marked <!-- base: c6719af278267196 -->
Each file under `crates/clm-init/` holding a test, an integration test under `tests/` or a unit-test module under `src/`, that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, a page under `docs/guide/` or `docs/spec-format.md`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`). A test that reads the guide's pages as a directory, such as the one that checks every page is in the compiled table, SHALL mark `docs/guide/**`.

#### Scenario: CLM's own ARCHITECTURE.md
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `ARCHITECTURE.md` as the only differing path
- THEN it selects `crates/clm-init/tests/says_component.rs`

#### Scenario: the guide's pages
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/quickstart.md` as the only differing path
- THEN it selects `crates/clm-init/src/guide.rs`, whose tests compare each page's file with the compiled table, and `crates/clm-init/src/guide/entry_pages.rs`, whose test reads the quickstart

#### Scenario: the spec format page
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/spec-format.md` as the only differing path
- THEN it selects `crates/clm-init/src/guide.rs`

#### Needs
- check.affected/affected-readers
