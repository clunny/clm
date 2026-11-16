---
title: "Two doc-readers scenarios name the guide tests' old file"
kind: deferred
components: [clm]
---

Two scenarios of `clm/doc-readers-marked` say a change to a guide page or to `docs/spec-format.md` selects `crates/clm/tests/guide.rs`. `init`'s build moves the tests of `clm guide` into `crates/clm-init/tests/guide.rs` (`commands-live-in-their-crates`), and the root's `doc_readers.rs` now accepts either file so that the move can land. Once it has, the scenarios as written are false.

Due with the batched spec change that follows the phase 4 build run (`backlog/seventeen-init-tests-can-t-move-into-its-crate`): reword both scenarios to name `crates/clm-init/tests/guide.rs`; the root's next build then asserts that file alone again.
