---
title: "Building the renamed-test rule breaks land's test of the old rule, and land can't move first"
kind: question
components: [check.coverage, land]
origin: 20261118-renaming-a-citing-test
blocks: [check.coverage/test-lists]
choices:
  - "Coverage build edits the land test"
  - "Land moves its test first"
  - "Expand-contract the rule"
---

Change `20261118-renaming-a-citing-test` has `check.coverage/test-lists` (`bbdcfe2f`) make `deleted_tests` return nothing for a test renamed in its own file that keeps citing what it cited, and has `land/deleted-tests-approved` wait on it through `#### Needs`, so `check.coverage` builds first.

But `crates/clm-land/tests/deleted_tests.rs::a_renamed_test_refuses_the_land`, `land`'s file, asserts the old rule: its fixture (`tests/t.rs` holding `one` and `two` citing `a/x` on `main`, and `one` and `second` citing `a/x` on branch `tidy`) is exactly the new scenario *a renamed test*, and it expects `clm land tidy` to refuse. Any build of `test-lists` makes `deleted_tests` return nothing there, so the land lands and that test fails. The change's own notes plan to re-cite that test to *a renamed test that drops its citation* in `land`'s build, which the `Needs` puts after this one. A `check.coverage` build may not edit `land`'s file (`path-ownership`, `coupled-changes-sequence`), and `land` isn't offered until `test-lists` is current, so neither build can go first while keeping `main` green.

Which spec change sequences it?

- **Let `check.coverage`'s build edit that one land test**: an exception, like `capability-tests-follow-rules`, letting it take the marker off `second` in the fixture (so the test proves *a renamed test that drops its citation*, under its true name) in the same branch.
- **Drop land's Need**: a requirement of `land`, without the `Needs`, moves the fixture to drop the marker first; `test-lists` then builds, and `land/deleted-tests-approved` follows.
- **Expand-contract**: `test-lists` offers the renamed-aware rule beside the old one (`clm:remove-by`), `land` moves to it, then the old goes.
