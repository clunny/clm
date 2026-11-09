---
component: clm
---

## MODIFIED Requirements

### Requirement: doc-readers-marked <!-- base: 055977ee1d399ee9 -->
Each file of a crate under `crates/` that holds a test, an integration test under the crate's `tests/` or a unit-test module under its `src/`, that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md`, a file under `specs/`, a page under `docs/guide/` or `docs/spec-format.md`, whether it opens the file or runs a command that reads it in this repository or prints it compiled in (`clm guide`), or that reads the repository's own `clm.toml`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`) or counts as checked because only `clm.toml`'s comments or upgrade notes changed (`land/settings-unchanged-is-checked`). A test that reads the guide's pages as a directory, such as the one that checks every page is in the compiled table, SHALL mark `docs/guide/**`. The rule is the root's, whatever crate holds the test, and its scenarios are proven by tests under `crates/clm/tests/` that run `clm-affected`'s `readers` on this repository's markers, since `clm` depends on every component and a crate below `check.affected` can't (`doc-readers-one-rule`).

#### Scenario: the retired-terms test reads the specs
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `specs/SPEC.md` as the only differing path
- THEN it selects `crates/clm/tests/retired_terms.rs`

#### Scenario: the root documents' readers
- GIVEN the same markers
- WHEN `readers` runs on them with `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md` as the differing paths
- THEN it selects `crates/clm/tests/retired_terms.rs` and `crates/clm/tests/readme.rs`

#### Scenario: other crates' readers of ARCHITECTURE.md
- GIVEN the same markers
- WHEN `readers` runs on them with `ARCHITECTURE.md` as the only differing path
- THEN it selects `crates/clm-init/tests/says_component.rs`, `crates/clm-model/tests/carried.rs` and `crates/clm-context/tests/carried.rs`

#### Scenario: the readers of clm.toml
- GIVEN the same markers
- WHEN `readers` runs on them with `clm.toml` as the only differing path
- THEN it selects `crates/clm/tests/cli.rs`, whose allow-list test reads `clm.toml`'s comments

#### Scenario: the guide's readers
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/glossary.md` as the only differing path
- THEN it selects `crates/clm/tests/glossary.rs`, `crates/clm/tests/guide.rs` and `crates/clm/tests/retired_terms.rs`

#### Scenario: the guide's readers in other crates
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/quickstart.md` as the only differing path
- THEN it selects `crates/clm-init/src/guide.rs`, whose tests compare each page's file with the compiled table, and `crates/clm-init/src/guide/entry_pages.rs`, whose test reads the quickstart

#### Scenario: the settings page's reader
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/settings.md` as the only differing path
- THEN it selects `crates/clm-config/tests/settings_guide.rs`

#### Scenario: the spec format's readers
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/spec-format.md` as the only differing path
- THEN it selects `crates/clm/tests/guide.rs`, `crates/clm/tests/retired_terms.rs` and `crates/clm-init/src/guide.rs`

#### Needs
- check.affected/affected-readers
