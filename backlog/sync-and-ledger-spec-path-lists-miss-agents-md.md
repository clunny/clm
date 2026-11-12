---
title: "The sync and ledger spec-path lists don't name AGENTS.md"
kind: bug
components: [sync, ledger]
---

Found by the review of the `init` build that makes `clm init` write `AGENTS.md`: `crates/clm-sync/src/lib.rs` (its spec-path list) and `crates/clm-ledger/src/doc_only.rs` (its two doc-only lists) name `CLAUDE.md` but not `AGENTS.md`, while the gate, check, health and survey now count `AGENTS.md` as a spec path. An edit to a project's `AGENTS.md` is then handled as code by sync and counted in the ledger's code-key fingerprint. Expected: `AGENTS.md` is a spec path wherever `CLAUDE.md` is (`platform`, `work-from-any-agent-harness`).
