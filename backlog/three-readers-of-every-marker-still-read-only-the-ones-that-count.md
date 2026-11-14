---
title: "dangling-markers, the rename rewrite and test selection read only the markers that count, against their requirements"
kind: bug
components: [check, rename, check.affected]
origin: 20261114-proof-is-real
blocks: [check.coverage/markers-in-tests]
---

`check/dangling-markers`, `rename/marker-rewrite` and `check.affected/affected-files` each say they read every marker, `scan`'s `outside` included (`check.coverage/test-spans`). Their code reads only the markers that count:

- `crates/clm-check/src/lib.rs` passes `markers.req` alone to `dangling::violations`.
- `crates/clm-rename/src/build.rs` builds its file list and `holding` from `scan(root).req`.
- `crates/clm-affected/src/lib.rs` maps files to requirements through `clm_check::markers_in`.

Each requirement reads as current, because the test its build added passes while `req` still holds every marker: until `check.coverage/markers-in-tests` is built, a marker outside a test is in both lists, so a test that writes one into a fixture can't tell code that reads `outside` from code that doesn't. The `check.coverage` build that narrowed `req` saw the three tests fail (`dangling_marker_outside_a_test_is_reported_once`, `a_marker_outside_a_test_is_rewritten`, `a_marker_outside_a_test_still_selects`) and reverted.

The requirements are right as written, so this is a code fix (`code-fix-resolves-bug`), on one branch while the root sets no `wiring`: each fix starts with a test that hands the code a marker present only in `outside`, which fails until the code reads that list. It replaces the question `dangling-markers-rename-and-affected-still-read-only-the-markers-that-count`, which the human answered on 2026-11-14: no spec change is needed.

`check`'s `cites` and `skipped` rules and the mutation pass read the markers that count on purpose (`check.coverage`'s Design, "Who reads which markers").
