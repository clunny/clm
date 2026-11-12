---
component: web.worker.platform
---

## MODIFIED Requirements

### Requirement: code-key-doc-only <!-- base: 609e1b0d7fe27821 -->
`keys::code_key` SHALL give, for a tree listing, the key `clm_ledger::code_key` gives for the same tree, leaving out exactly the entries whose path `clm_ledger::doc_only` (`ledger/doc-only`) answers true for, by calling it, so a change to `ledger`'s list reaches the Worker's code keys with no edit to this component.

#### Scenario: doc-only paths from the ledger
- GIVEN a tree listing of `blob` entries (any mode and sha) at `docs/x.md`, `docs/guide/intro.md`, `backlog/y.md`, `CLAUDE.md`, `AGENTS.md`, `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `keys::code_key` runs on it
- THEN it equals `keys::code_key` over only `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`, the entries `clm_ledger::doc_only` answers false for, and differs from the key of the listing with any one of those three removed

#### Scenario: the guide's files are doc-only
- GIVEN a repository whose commits `a`, `b` changing only `docs/x.md`, `c` changing only `docs/guide/start.md`, `d` changing only `docs/spec-format.md` and `e` changing only `crates/a/src/lib.rs`, each listed as the Trees API lists it
- WHEN `keys::code_key` runs on each listing
- THEN each key equals `clm_ledger::code_key` of the same commit, `a`, `b`, `c` and `d` share one, and `e`'s differs from it

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

#### Needs
- ledger/doc-only
