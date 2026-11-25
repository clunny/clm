---
title: "land's deleted-test refusal advises a rename the count may still refuse"
kind: bug
components: [land]
---

`land/deleted-tests-approved`'s refusal (00650f33) advises "rename it in its file to a name no test carried at the merge base, citing every requirement it cited". `check.coverage`'s `deleted_tests` (crates/clm-coverage/src/lists.rs, the `room` count) also caps a file's renames at its new tests less the unmarked tests the file lost. So a branch that renames cited `two` to `second`, keeping its marker, and deletes an uncited `helper` in the same file is refused, though it did exactly what the advice says: a drafter following the message loops, or lists a rename as a deletion, the misreading 20261124-clm-land-s-refusal set out to end. Either the advice names the cap (e.g. "with no uncited test of that file deleted") or the count changes; the text is quoted in the requirement, so it needs a spec change. Found by /code-review on impl/land. The same review found the advice's phrase "even another than it cited" awkward (e.g. "even one it didn't cite").
