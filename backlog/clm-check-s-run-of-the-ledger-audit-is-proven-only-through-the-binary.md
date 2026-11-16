---
title: "clm check's run of the ledger audit is proven only through the binary"
kind: deferred
components: [clm, ledger]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
---

`ledger/commands-in-crate` moved 64 of the 65 tests `moved-tests.txt` lists for `ledger` into `crates/clm-ledger/tests/ledger.rs`. One stays in `crates/clm/tests/ledger.rs`: `check_runs_verify_on_main`, citing `ledger/verify`, whose scenario "check runs it on main" runs `clm check`, another component's command, on what the ledger made. `clm-ledger` can't call `clm-check` (it doesn't depend on `check`), and only the root's tests get the binary, so the test runs the binary as before (`commands-live-in-their-crates`). `clm-ledger` now offers the audit as `on_main(root)`, which `crates/clm` passes to `clm-check`'s call as `CheckInputs::more`; this test is what proves that wiring.

What is needed: the spec change that moves such scenarios into requirements of the root, so the test cites the root's requirement and `crates/clm/tests/` holds only the root's tests (`clm/routes-only`).

## The human's answer (2026-11-15)

The first choice, under the standing answer for tests of this kind (`commands-live-in-their-crates`): a requirement of the root states that `clm check` runs the ledger's audit on `main`, since `crates/clm` is what passes one crate's result to the other's call. The batched spec change that follows the build run moves the scenario there and deletes this item.

Answered, so no longer a question: it waits only for the batched spec change, which is due when the phase 4 build run has drained.
