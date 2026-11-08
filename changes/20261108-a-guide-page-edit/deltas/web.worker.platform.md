---
component: web.worker.platform
---

## MODIFIED Requirements
### Requirement: code-key-doc-only <!-- base: 09f136f11d33d416 -->
`keys::code_key` SHALL leave out of a commit's code key exactly the entries whose path `clm_ledger::doc_only` (`ledger/doc-only`) answers true for, by calling it, and `crates/clm-web-platform/src/keys.rs` SHALL hold no list of doc-only paths of its own, so a change to `ledger`'s list reaches the Worker's code keys with no edit to this component.

#### Scenario: doc-only paths from the ledger
- GIVEN a tree listing of `blob` entries (any mode and sha) at `docs/x.md`, `docs/guide/intro.md`, `backlog/y.md`, `CLAUDE.md`, `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `keys::code_key` runs on it
- THEN it equals `keys::code_key` over only `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`, the entries `clm_ledger::doc_only` answers false for, and differs from the key of the listing with any one of those three removed

#### Scenario: no copy of the list
- GIVEN the text of `crates/clm-web-platform/src/keys.rs`, read by a test outside it with `include_str!`
- WHEN the tests run
- THEN the test fails, naming each it finds, when the text holds any of the string literals `"specs/"`, `"changes/"`, `"backlog/"`, `"docs/"`, `"docs/guide/"`, `"ARCHITECTURE.md"`, `"CAPABILITIES.md"`, `"README.md"` or `"CLAUDE.md"`

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

#### Needs
- ledger/doc-only

### Requirement: code-key-as-ledger <!-- base: 1e29d98bc02e9e47 -->
`keys::code_key` SHALL give, for a tree listing, the key `clm_ledger::code_key` gives for the same tree, leaving out exactly the paths `ledger`'s `doc_only` names: a tree that differs from another only in `docs/x.md`, only in `docs/guide/intro.md` or only in `docs/spec-format.md` gets the same key, and one that differs only in `crates/a/src/lib.rs` gets a different key.

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

#### Needs
- ledger/doc-only

#### Scenario: the guide's files are doc-only
- GIVEN a repository whose commits `a`, `b` changing only `docs/x.md`, `c` changing only `docs/guide/start.md`, `d` changing only `docs/spec-format.md` and `e` changing only `crates/a/src/lib.rs`, each listed as the Trees API lists it
- WHEN `keys::code_key` runs on each listing
- THEN each key equals `clm_ledger::code_key` of the same commit, `a`, `b`, `c` and `d` share one, and `e`'s differs from it
