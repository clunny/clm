---
title: "A land judges whether a test's name is old from the files the branch touches alone"
kind: bug
components: [land, check.coverage]
---

Found by the code review of the build of `check.coverage/test-lists@1f461b0a`.

`check.coverage/test-lists` makes a head test's name old, so no rename, when any test of `base_files` carries it. `crates/clm-land/src/deleted_tests.rs` passes `deleted_tests` the base and head texts of the files the branch touches only, so a name a test in an untouched file carries still counts as new there.

Untouched `tests/u.rs` holds `kept`. The branch deletes `two` from `tests/t.rs` and adds a new test `kept` there citing what `two` cited. At the land `kept` counts as a rename of `two`, though over the whole tree it is a name the base already held.

Either the land passes every test file's base text (at some cost), or `land/deleted-tests-approved` states the narrower scope. Which is right is a design question for the `land` component.
