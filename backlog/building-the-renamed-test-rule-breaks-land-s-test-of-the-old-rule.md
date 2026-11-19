---
title: "Building the renamed-test rule breaks land's test of the old rule, and land can't move first"
kind: question
components: [check.coverage, land]
origin: 20261118-renaming-a-citing-test
blocks: [check.coverage/test-lists]
choices:
  - "Small branch edits the land fixture"
  - "Coverage build may edit the test"
  - "Drop land's Need"
  - "Expand-contract the rule"
---

Change `20261118-renaming-a-citing-test` has `check.coverage/test-lists` (`bbdcfe2f`) make `deleted_tests` return nothing for a test renamed in its own file that keeps citing what it cited, and has `land/deleted-tests-approved` wait on it through `#### Needs`, so `check.coverage` builds first.

But `crates/clm-land/tests/deleted_tests.rs::a_renamed_test_refuses_the_land`, `land`'s file, asserts the old rule: its fixture (`tests/t.rs` holding `one` and `two` citing `a/x` on `main`, and `one` and `second` citing `a/x` on branch `tidy`) is exactly the new scenario *a renamed test*, and it expects `clm land tidy` to refuse. Any build of `test-lists` makes `deleted_tests` return nothing there, so the land lands and that test fails. The change's own notes plan to re-cite that test to *a renamed test that drops its citation* in `land`'s build, which the `Needs` puts after this one. A `check.coverage` build may not edit `land`'s file (`path-ownership`, `coupled-changes-sequence`), and `land` isn't offered until `test-lists` is current, so neither build can go first while keeping `main` green.

How should it be sequenced?

- **Small branch edits the land fixture** (no spec change): a branch outside `impl/` (`small-change-no-claim`), run by an attended session, takes the marker off `second` in that fixture. The test then refuses under both the old and the new rule, under a name that stays true, so `main` stays green and `test-lists` can build; `land`'s own build re-cites it to *a renamed test that drops its citation* later. Whether a fixture edit that changes which scenario a test proves fits `small-change-no-claim` is the judgement this asks for.
- **Coverage build may edit the test**: a spec change giving `check.coverage`'s build an exception, like `capability-tests-follow-rules`, to make that same fixture edit in its own branch.
- **Drop land's Need**: a spec change in which a requirement of `land`, without the `Needs`, moves the fixture first; `test-lists` then builds, and `land/deleted-tests-approved` follows.
- **Expand-contract**: `test-lists` offers the renamed-aware rule beside the old one, as a new function since the change keeps `deleted_tests`' signature (`clm:remove-by`), `land` moves to it, then the old goes.

Once it is settled, delete `backlog/workaround-check.coverage-test-lists-1.md` and `backlog/reconsider-check.coverage-test-lists.md`, which this blocker alone raised.
