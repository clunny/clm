---
title: "`check` is split by carving coverage out as `check.coverage`, which `clm-check` re-exports"
date: 2026-10-27
status: accepted
---

`check` was over its code budget (50,797 of 40,000 tokens on 2026-10-27, `backlog/split-check`), and `20261027-check-hardening` adds five rules to it. Raising the budget was rejected: the node holds two parts that change for different reasons, which is what `slim-nodes` splits on.

- **Coverage**: the markers, each requirement's state, the pending list, `clm status`'s sections and which changes are complete. The queue, the views, `clm sync`, `clm checkpoint` and the `web` Worker read it; it must build for wasm; it changes when the reconciliation model does (`node-reconciliation`, `archive-on-sync`, `requirement-needs`).
- **Rules**: what `clm check` fails on, the detectors and `clm size`. Only the `clm` commands call them; they read a checkout; they change when the method gains a rule.

So the coverage half becomes the child `check.coverage` (crate `clm-coverage`) with `coverage-report`, `markers-from-files`, `pending-report` and `status-report`, and `check` keeps its ID, its rules and the node map's description (`nested-splits`). The definitions of the states stay in `check`'s Contract, because the nodes that read coverage depend on `check` and a child's Contract isn't in their context, while the child reads its parent's Contract on its spine.

`clm-check` depends on `clm-coverage` and re-exports its library unchanged. Every consumer (`orchestrator`, `orchestrator.run`, `orchestrator.checkpoint`, `sync`, `view`, `web.worker`, the root) keeps its `clm_check::` imports and its `depends_on: [check]`, so the split needs no requirement on any of them; only the text that named `check/pending-report` and `check/status-report` changes. Making each consumer import `clm-coverage` directly was rejected: seven nodes' builds and `depends_on` edits for no change in behaviour.

The parent depends on the child, so the child builds first (`nested-splits`): its build creates `crates/clm-coverage`, moves the modules and their tests in, deletes `clm-check`'s copies, adds the re-exports, and cites its requirements, re-citing the tests that cited the removed `check/` IDs. `check`'s build then cites what is left and moves the CLI tests of the moved requirements out of `crates/clm/tests/cli.rs` into a file of their own, so `cli.rs` counts toward `check` alone.

Expected sizes after both builds, estimated from the files: `check.coverage` about 30,000 code tokens, `check` about 25,000 before the new rules and 35,000 after.
