---
title: "clm's CLI integration tests prove other components' requirements through the root's binary"
kind: idea
components: [clm]
---

## Intent

`crates/clm/tests/*.rs` sit in the root's `paths` because Cargo gives the built binary only to the root package's tests, yet most files prove another component's requirements (`ledger.rs`, `run.rs`, `plugin_*.rs`) by running the whole CLI. That is the upward test `temporary-exemptions` says to refactor away: each component should prove its requirements through its own surface. So each crate offers a library entry point (`run(args, io)`, as several already do) whose tests live in the crate, and the binary becomes a thin dispatcher with its own tests of dispatch only.

Until then, the root's tests keep relying on `path-ownership`'s transitional re-citing exception.

## Open questions

- Which commands have no library entry yet, and what does each need (its I/O, its environment) to be called in-process?
- Do any tests need the real binary (process exit codes, signals, `closed_stdout.rs`)? Those stay the root's, as tests of the CLI itself.
