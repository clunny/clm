---
status: done
resolves:
  - clm-land-s-refusal-of-a-deleted-test-reads-as-if-a-test-keeping
---

## Intent

When `clm land` refuses a branch that loses a citing test (`land/deleted-tests-approved`), its message reads "Keep the test under its name, or under a name no citing test carried, in its file, citing every requirement it cited". Read plainly, the last clause covers both choices, so a test that keeps its name would have to keep every citation, and re-citing it to another requirement would count as a deletion. That is not what the land checks: `check.coverage`'s `deleted_tests` counts a file's citing tests of a name whatever requirements they cite, so a test re-cited under its name is not gone, and it asks "every requirement it cited" only of a test renamed in its own file. In wave 6 a drafter read the message as written and listed 7 re-cited tests in `deleted-tests.txt`, which would have asked the human to approve cuts that weren't cuts.

The message's text is stated in the requirement, so the fix is a spec change.

## Approaches

- **Reword the message and state the rule in the requirement** (taken). The message splits its choices with semicolons, so each carries its own condition: keep the test under its name, citing any requirement; or rename it in its file to a name no test carried at the merge base, citing every requirement it cited; or leave the requirement unbuilt and file a question. The requirement adds one sentence saying a test re-cited under its name loses no test, deferring to `check.coverage/test-lists` for the counting (its exception, a cited test whose unmarked namesake in its file may have gained a marker in its place, stands), and a scenario, *a re-cited test*, proving it at the land.
- **Make the land refuse a re-cited test that drops a citation.** Rejected: re-citing a test to the requirement that now states what it proves is what builds are told to do (`AGENTS.md`, step 4), and a re-cite moves no behaviour out of the spec, so it is no cut (`removal-needs-a-yes`).

The old message's "a name no citing test carried" was also inexact: `deleted_tests` refuses a new name any test of the merge base carried, cited or not. The new text says "no test carried at the merge base".

## Outcome

- `land/deleted-tests-approved` prints the reworded line and states that re-citing a test under its name loses no test; its new scenario *a re-cited test* lands a branch whose test keeps its name and cites `b/y` in place of `a/x`.
- `land`'s build changes the message in `crates/clm-land/src/deleted_tests.rs` and the expected line in `crates/clm-land/tests/deleted_tests.rs`, re-cites the tests citing the requirement (in both files, `lists_live_and_archived` included), and adds a test for *a re-cited test*.
- No behaviour changes; `check.coverage` is untouched.

## What the requirements no longer state

- `land/deleted-tests-approved`'s first sentence changes only the text of the line it prints: "Keep the test under its name, or under a name no citing test carried, in its file, citing every requirement it cited, or leave the requirement unbuilt and add a question item" becomes "Keep the test under its name, citing any requirement, even another than it cited; or rename it in its file to a name no test carried at the merge base, citing every requirement it cited; or leave the requirement unbuilt and add a question item". What the land refuses is unchanged.

## Open questions

None.
