---
title: "A test target of several files is never judged by its own mutants"
kind: idea
components: [ci]
---

`ci/mutate-script` judges a Rust block by the mutants of its test files only when its tests run no line of any other Rust file of the repository but the files of its `test` lines. A test target made of several files, such as `crates/clm/tests/ci_suite/` (`main.rs` and its modules) or one that uses `tests/common/mod.rs`, always runs lines of a helper module no `test` line names. So a requirement whose rule is code in such a target's own file falls back to the code path and reads `survived`, or `not-judged` with a `reads` line: the gap the test-file path was added to close. The code review of the build that added the path (2026-11-19) raised it.

A spec change could count the other files of the same test target, the files under the same crate's `tests/`, as the block's own, while a file of another crate still sends the block to the code path. It should weigh that a shared helper module's mutants are caught however little a test asserts, which is why the path tries no helper beside the code's mutants.
