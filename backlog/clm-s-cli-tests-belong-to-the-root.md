---
title: "clm's CLI integration tests belong to the root component although each file proves another component"
kind: idea
components: [clm]
---

## Intent

`crates/clm/tests/*.rs` sit in the root's `paths` because Cargo gives the built binary only to the root package's tests, yet most files prove one other component (`ledger.rs`, `run.rs`, `plugin_*.rs`). So builds of those components rely on `path-ownership`'s transitional re-citing exception and can't add a test file of their own. With `tests` and `test_depends_on` (`components-own-their-tests`), each component can own its files there and depend on the root for the binary.

## Open questions

- Which files cite several components (`cli.rs`, `affected.rs`, `checkpoint.rs` and others also cite `clm`), and how should each be split?
- One file per component (`crates/clm/tests/<component>.rs`), or a directory per component, given that Cargo builds each top-level test file as its own binary?
