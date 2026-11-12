---
status: applied
resolves: [sync-and-ledger-spec-path-lists-miss-agents-md]
---

## Intent

Delete `backlog/sync-and-ledger-spec-path-lists-miss-agents-md`, which is already fixed. The change `20261112-any-harness` added the rules it asks for (`sync`'s requirement that `clm autosync` snapshots `AGENTS.md` as it does `CLAUDE.md`, and `AGENTS.md` in `ledger/doc-only` and `code_key`'s requirement), and their builds landed on `main` as `07f76e37` ("Treat AGENTS.md as doc-only in the ledger's code key") and `0f9aac83` ("autosync: snapshot AGENTS.md as a spec path"): `crates/clm-sync/src/lib.rs` and `DOC_ONLY` in `crates/clm-ledger/src/doc_only.rs` both name `AGENTS.md`. The second list the item counts in `doc_only.rs`, `LEGACY_DOC_ONLY`, is frozen on purpose (`ledger/legacy-doc-only`, `legacy-code-key`): it records the list as it was when the keyless `landed` records were written, so it rightly leaves `AGENTS.md` out.
