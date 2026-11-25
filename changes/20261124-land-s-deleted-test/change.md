---
status: proposed
resolves:
  - land-s-deleted-test-refusal-advises-a-rename-the-count-may-still
---

## Intent

When `clm land` refuses a branch that loses a citing test (`land/deleted-tests-approved`), its advice says a test may be renamed in its file to a name no test carried at the merge base, citing every requirement it cited. That is not enough: `check.coverage/test-lists` lets each uncited test a file loses take one of the file's new tests first, since it could have become any of them, so a branch that renames cited `two` to `second`, keeping its marker, and deletes an uncited `helper` in the same file is refused although it did what the advice says. A drafter following the message loops, or lists the rename as a deletion, the misreading `20261124-clm-land-s-refusal` set out to end. The advice's text is stated in the requirement, so the fix is a spec change.

The same review found the advice's "even another than it cited" awkward; it becomes "even one it didn't cite".

## Approaches

- **Name the cap in the advice** (taken). The rename choice gains "and delete no uncited test of that file on the same branch", a sufficient condition a drafter can follow, matching what `docs/guide/how-it-works.md` already tells the human (delete the uncited test on a branch of its own, keep the citing test's name, or list it). The requirement says why, deferring to `check.coverage/test-lists` for the counting, and a new scenario, *a rename beside a deleted uncited test*, shows the refusal.
- **Change the count so the rename passes.** Rejected: the cap is deliberate. The trees can't tell `two` renamed to `second` beside `helper` deleted from `helper` renamed to `second` as it gains `two`'s marker beside `two` deleted, and the second loses the behaviour `two` proved with no list naming it (`check.coverage`'s scenario *an uncited test deleted beside a rename*, `removal-needs-a-yes`). Reading the bodies to tell them apart is a heuristic, not a checked rule.

## Outcome

- `land/deleted-tests-approved` prints the reworded advice and says why a rename beside a deleted uncited test is refused; its new scenario *a rename beside a deleted uncited test* proves the refusal at the land.
- `land`'s build changes the message in `crates/clm-land/src/deleted_tests.rs` (and its module doc's summary of the advice, `:5`), the expected line in `crates/clm-land/tests/deleted_tests.rs`, re-cites the tests citing the requirement, and adds a test for the new scenario.
- What the land refuses is unchanged; `check.coverage` is untouched.

## What the requirements no longer state

- `land/deleted-tests-approved`'s first sentence changes only the text of the advice it prints: "Keep the test under its name, citing any requirement, even another than it cited; or rename it in its file to a name no test carried at the merge base, citing every requirement it cited; or leave the requirement unbuilt and add a question item" becomes "Keep the test under its name, citing any requirement, even one it didn't cite; or rename it in its file to a name no test carried at the merge base, citing every requirement it cited, and delete no uncited test of that file on the same branch; or leave the requirement unbuilt and add a question item". What the land refuses is unchanged.

## Open questions

None.
