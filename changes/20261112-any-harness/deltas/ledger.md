---
component: ledger
---

## MODIFIED Requirements

### Requirement: doc-only <!-- base: 780e373b73be8cb3 -->
`clm-ledger` SHALL offer `doc_only(path)`, true for a path relative to the repository root under `specs/`, `changes/`, `backlog/` or `docs/`, `docs/guide/` and `docs/spec-format.md` included, or equal to `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` or `AGENTS.md`, and false for any other path, with its default features disabled as well as enabled. `code_key` SHALL leave out exactly the entries `doc_only` names, so the list is written once.

#### Scenario: paths
- GIVEN the paths `docs/x.md`, `docs/guide/intro.md`, `docs/spec-format.md`, `backlog/y.md`, `CLAUDE.md`, `AGENTS.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `doc_only` runs on each
- THEN it returns true for `docs/x.md`, `docs/guide/intro.md`, `docs/spec-format.md`, `backlog/y.md`, `CLAUDE.md` and `AGENTS.md`, and false for `crates/a/src/lib.rs` and `docsy/z.md`

#### Serves
- capability/land-safely
- architecture/deterministic-enforcement

### Requirement: code-key <!-- base: b3dc8cf840f779fc -->
`code_key(root, rev)` SHALL print the same key for two commits whose trees differ only under `specs/`, `changes/`, `backlog/` or `docs/`, the guide's pages under `docs/guide/` and `docs/spec-format.md` included, or in `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` or `AGENTS.md`, and a different key when any other entry differs, a new top-level file included.

#### Scenario: doc-only difference
- GIVEN commits `a`, `b` that changes only `docs/x.md` and `backlog/y.md`, `c` that also changes `crates/a/src/lib.rs`, `d` that adds `tool.toml` at the root, `e` that changes only `docs/guide/intro.md`, and `f` that changes only `docs/spec-format.md`
- WHEN `code_key` runs on each
- THEN `a`, `b`, `e` and `f` give the same key, and `c` and `d` each give a different one

#### Serves
- capability/land-safely
- architecture/deterministic-enforcement
