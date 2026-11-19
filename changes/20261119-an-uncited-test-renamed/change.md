---
status: proposed
resolves: [an-uncited-test-renamed-as-it-gains-a-marker-hides-a-deleted-test]
---

## Intent

A test the base held with no marker, renamed in its file as it gains one, can no longer stand for a deleted citing test, so the land refuses that deletion as it refuses one hidden by a test that keeps its name (`backlog/an-uncited-test-renamed-as-it-gains-a-marker-hides-a-deleted-test`). `check.coverage/test-lists` is reworded; the decision `renamed-test-keeps-its-citations` is amended to match.

## Problem

- `20261119-a-test-that-gains` made a head test's name new only when no base test carries it, cited or not, so an unmarked test that keeps its name and gains a marker no longer hides a deleted test. One renamed at the same time still does: its new name no base test carries, so it *fits* the deleted test.
- At the base `tests/t.rs` holds `one` and `two`, each citing `a/x`, and `quiet`, with no marker. A branch deletes `two` and renames `quiet` to `quiet_now`, citing `a/x`. `deleted_tests` returns nothing, and `clm land` accepts the deletion with no `deleted-tests.txt` naming it (`removal-needs-a-yes`).
- The code does what the requirement states, so the fix is a spec change (`code-fix-needs-a-written-rule`). Nothing since 2026-11-12 fixed it: the item was filed today by the review of `test-lists@1f461b0a`'s build, and no code fix resolves it.

## Approaches

- **Taken: an unmarked base test that is gone from the head takes a new name before a citing test does.** The trees can't tell which base test a new name stands for, and an unmarked test that left its file is as likely a source as a deleted citing one; giving it the new name first errs toward refusing, as every other unclear case of `test-lists` does. Since an unmarked test cites nothing, it could have become any new test of its file, so the rule is a count: the tests counted renamed in a file number no more than its new tests less its gone unmarked tests. A new test here is one of the head's file whose bare name no base test carries, holding a marker or not, so an unmarked test renamed to another unmarked name uses up a new name nobody cites, and a rename of a citing test beside it still passes. The pairing stays a largest matching (a truncation of the transversal matroid the greedy pass already walks), so taking the gone tests in order still renames as many as can be. The build is a cap per file on the existing pass.
- **Rejected: accept the gap.** The previous change closed the same hole for a test that keeps its name; leaving the renamed variant open leaves `removal-needs-a-yes` to whether a build happens to rename the test it un-hides, and the fix is one clause.
- **Rejected: count only an unmarked test whose new name holds a marker.** It would refuse less, but the new names are what the trees show; which unmarked test became which new name is the same guess the matching already declines to make.

## Outcome

- `check.coverage/test-lists` (modified): defines a *new* test of a file (bare name no base test carries, marker or not) and a gone *unmarked* test (no marker at the base, bare name no test of `head_files` carries), and caps the tests counted renamed in a file at its new tests less its gone unmarked tests. Three scenarios added: *an uncited test renamed as it gains its first marker* (the item's case, reported gone), *an uncited test renamed to a name it doesn't cite under* (nothing returned) and *an uncited test deleted beside a rename* (the rename reported gone).
- `docs/decisions/renamed-test-keeps-its-citations.md` states the rule as amended.

## Constraints

- `deleted_tests`' signature and return type stay as they are; `clm land`, `clm land --check-only` and `clm coverage --deleted-tests` change only through what it returns. `land/deleted-tests-approved`'s message ("under a new name in its file citing every requirement it cited") still reads right.
- Every scenario of `test-lists` as it stands keeps its outcome: none has a gone unmarked test.

## Edge cases and workarounds

- A branch that deletes an unmarked test and renames a citing test in the same file is refused, though nothing was hidden (*an uncited test deleted beside a rename*). It errs toward refusing; the build deletes the unmarked test on a branch of its own, keeps the citing test's name, or the change lists it. An unmarked test is rarely deleted on a branch that renames its neighbour.
- `clm land` passes only the files the branch touches as `head_files`, so an unmarked test moved to a file the branch doesn't otherwise touch is in both sides' files only because the move touches both; a move is not counted gone.

## Open questions

None.
