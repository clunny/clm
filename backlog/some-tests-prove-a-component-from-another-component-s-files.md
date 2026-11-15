---
title: "Some tests prove a component from another component's files"
kind: deferred
components: [view, check.coverage, upgrade, land.approval, apply, view.repo, land]
origin: 20261115-tests-in-their-own-files
blocks: [clm/routes-only, gate/own-files]
---

Beside the tests that `commands-in-crate` moves, these prove a requirement from a file its component doesn't own, which only the clause "a file that cites the component" lets a build change (`own-files-at-the-gate`). Measured on `main` at `4f96740b`:

- **In the root's top-level test files, through the binary**, listed in `changes/20261115-commands-in-their-crates/moved-tests.txt` (under `changes/archive/` once that change is complete): `view` (49 tests, in `status.rs` and `view.rs`), `check.coverage` (14, in `cli.rs`, `pending.rs` and `status.rs`) and `upgrade` (1, in `upgrade.rs`). These components offer no command of their own, and the crates that offer `clm status`, `clm view`, `clm pending` and `clm coverage` lie above them. So each test calls its component's own crate with what the command gave it, a graph for `clm-view`, markers for `clm-coverage`, and asserts the same text on what the crate returns; that the command writes that text unchanged, on which stream and with which exit status, is proven by `view.repo/commands-in-crate` and `check/commands-in-crate`.
- **`check.coverage` in `check`'s crate**: 42 markers in `crates/clm-check/tests/coverage.rs` (7), `pending.rs` (20), `state_names.rs` (1) and `status.rs` (14). They move to `crates/clm-coverage/tests/` and call `clm-coverage`.
- **`land.approval` in `land`'s crate**: 25 markers in `crates/clm-land/tests/approval.rs` (10), `hold.rs` (7) and `signed.rs` (8). They move to `crates/clm-approval/tests/`.
- **Single markers**: `apply` in `crates/clm-sync/tests/sync.rs`, `view` in `crates/clm-view-repo/tests/plumbing.rs`, `view.repo` in `crates/clm-view/tests/views.rs`, and `land` in `crates/clm/tests/capability/upgrade_clm.rs`.

The work, one component at a time, each under its own `clm claim <component> --fix` for the component whose requirement the test cites, the last deleting this item: move each test into that component's own files under its own name, asserting what it asserted, and delete it where it was. Where a test proves two components at once, it stays with the owner of its file and the other component gets a test of its own. Where what a test asserts is the file owner's code and not the cited component's, say so in a `question` item naming the requirement: it may belong to the other component, which is a spec change. Delete no test that hasn't moved.

Due now. It changes nothing a requirement states.
