---
title: "The comment on clm coverage --deleted-tests states the old rule"
kind: bug
components: [check]
---

`crates/clm-check/src/lists.rs`'s doc comment on `deleted_tests_lines` says a `DELETED` line is printed for each citing test of `base` "that the working tree holds no test of that name for". Since `check.coverage/test-lists` at `1457e30b`, `clm_coverage::deleted_tests` counts tests by bare name across files: a test moved to another file under its name is not listed, and one that loses its last marker is.

Fix: reword the comment to defer to `clm_coverage::deleted_tests` (gone tests, counted by bare name). A code comment that contradicts a Contract is fixed by component with no test (`code-fix-needs-a-written-rule`). Found by the review of `impl/check.coverage`, whose build may not edit `check`'s files (`path-ownership`).
