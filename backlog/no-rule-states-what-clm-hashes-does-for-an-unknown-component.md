---
title: "No rule states what clm hashes does for an unknown component"
kind: question
components: [apply]
choices:
  - "Exit 2 with its message, as now"
  - "Exit 1 with its message"
---

`clm hashes nope` exits 2 and prints `clm hashes: no valid component `nope`` on standard error (`crates/clm-apply/src/command.rs`), and the test `hashes_fails_on_an_unknown_node` in `crates/clm-apply/tests/command.rs` checks it while citing `apply/hash-command`. Neither that requirement nor `apply`'s Contract says what happens for a component that doesn't exist, and `apply/commands-in-crate` gives status 2 only for arguments the call can't parse, which `nope` isn't. The spec-auditor found the gap in the build of `apply` that re-cited `hash-command` at `752d167d`. A spec branch should state the outcome in `hash-command` (or its Contract), so the test proves a written rule (`code-fix-needs-a-written-rule`).
