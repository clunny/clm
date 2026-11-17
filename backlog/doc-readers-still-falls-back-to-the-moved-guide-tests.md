---
title: "The doc-readers test still falls back to the removed root guide tests"
kind: bug
components: [clm]
---

`init`'s build of `commands-in-crate` moved the `clm guide` tests from `crates/clm/tests/guide.rs` to `crates/clm-init/tests/guide.rs` and deleted the root's file. `guide_tests()` in `crates/clm/tests/doc_readers.rs` still falls back to `crates/clm/tests/guide.rs` when the moved file holds no reads marker. That branch can no longer run, so it is dead transitional code (`no-dead-code`). It would also point the check at a file that no longer exists rather than failing. Expected: `guide_tests()` becomes a constant naming `crates/clm-init/tests/guide.rs`. The file is the root's, so `init`'s build left it.

A related cleanup: the root's tests in `crates/clm/tests/init.rs`, which cite only `clm/init-result-accepted` now, still assert everything `clm init` writes. `init` now proves that in its own crate, so each of those assertions exists twice. Those tests could be cut down to the clauses that run another command (`clm check`, `clm hashes`, `clm pending`, `clm setup`, `clm config`).
