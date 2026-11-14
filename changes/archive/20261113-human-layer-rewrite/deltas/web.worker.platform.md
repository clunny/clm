---
component: web.worker.platform
---

## MODIFIED Requirements

### Requirement: shared-layer <!-- base: 33f47bc23cf37cca -->
`clm-web-platform` SHALL hold the Worker's platform traits, HTTP types, access and permission checks, GitHub client, file fetches, tree keys, installation tokens, live state, ledger reading and web-ledger append, as the modules `platform`, `http`, `access`, `gh`, `files`, `keys`, `install`, `live` and `ledger`, and `clm-web` and `clm-web-review` SHALL use them from it, holding no module of those names of their own.

#### Scenario: one copy
- GIVEN the workspace
- WHEN a test in `clm-web-platform` lists `crates/clm-web/src`, `crates/clm-web-review/src` and `crates/clm-web-platform/src`, a directory that doesn't exist counting as holding nothing
- THEN `crates/clm-web-platform/src` holds `platform.rs`, `http.rs`, `access.rs`, `gh.rs`, `files.rs`, `keys.rs`, `install.rs`, `live.rs` and `ledger.rs`, and neither other directory holds a file of those names

#### Scenario: the append
- GIVEN the fake GitHub, a repo whose `clm/ledger/alice/web` holds one record, and an `Access` to it
- WHEN `ledger::append` commits a record by `alice/web`
- THEN the fake saw exactly four writes, a blob, a tree, a commit whose parent is the old tip, and a ref update with `force` false, and the branch's `records.jsonl` holds the old line then the record's

#### Serves
- architecture/reuse-first
- architecture/git-is-truth
