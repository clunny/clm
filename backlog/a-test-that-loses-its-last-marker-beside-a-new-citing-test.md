---
title: "A test that loses its last marker beside a new citing test counts as renamed"
kind: bug
components: [check.coverage]
---

Found by the code review of the build of `check.coverage/test-lists@d258ae73`.

`test-lists` says a test "that loses its last marker" is gone, but its counting rule lets such a test be *renamed*. Base `tests/t.rs` holds `one` and `two`, each citing `a/x`. The head holds `one`, `two` with no marker, and a new `second` citing `a/x`. The head holds no citing `two`, so `two` is taken; `second` is new and fits it, and the file has room for one rename (one new test, no gone unmarked test), so `deleted_tests` returns nothing and `clm land` accepts the stripped proof with no `deleted-tests.txt` (`removal-needs-a-yes`).

The code does what the counting rule states, and the rule contradicts the requirement's own summary sentence, so the fix is a spec change (`code-fix-needs-a-written-rule`): for instance, a taken test whose file at the head still holds a test of its bare name doesn't count as renamed.
