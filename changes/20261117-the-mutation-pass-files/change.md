---
status: applied
resolves: [the-mutation-pass-files-unproven-again-on-a-requirement-its]
---

## Intent

The mutation pass stops filing `unproven` on a requirement whose rule is data in files its tests read. `verify`'s plan names, on a `reads <glob>` line of each requirement's block, the globs of the `clm:reads` markers in the files holding its citing tests, read with `check.affected`'s `reads_in`. `.github/scripts/mutate.sh` then writes `not-judged`, instead of counting every mutant as survived, for a block whose tests run no line of its code with a mutant when the block holds a `reads` line. An empty test, in a file that reads nothing, is still `unproven`. This is plan work, as `20261116-the-mutation-pass-judges` was: the mutation pass is phase 3's evidence, and a false `unproven` item costs a build and a land of no code each time it is filed.

## Problem

`clm/doc-readers-marked` was filed `unproven` twice (`backlog/unproven-clm-eed96364`, `backlog/unproven-clm-09d807cc`), and both times a build found its tests sound and deleted the item (`0dff6fcd`, `0144210f`). Its rule is data: the `clm:reads` markers in test files of `clm-init`, `clm-model`, `clm-context`, `clm-config` and the root. Its tests, in `crates/clm/tests/doc_readers.rs`, run `clm-affected`'s `readers` over those files, and none of the root's code, which is all the block's `code` lines name. So `ci/mutate-script` takes the block for an empty test ("tests that run no line of any of its files that has a mutant") and counts every mutant survived. The clause written for data rules, tests that run lines of the code but none a mutant changes, doesn't reach it. The second filing was due by the spec (`14f04f5a` and `65e7b1dd` changed the tests in more than their markers), so it will recur whenever they change. The build that deleted `09d807cc` ran 22 hand mutants of the markers the scenarios name, and each failed a named test.

## Approaches

1. **The plan names what the tests read; the command reads it (taken).** `verify` already reads the citing tests' files and depends on `check.affected`, whose `reads_in` parses the marker, so a `reads <glob>` line costs a few lines and no new parser. Any project's mutate command, not only this repository's script, gets the signal. `mutate.sh` already ignores plan lines it doesn't know, so the verify side lands first without breaking the script, and `ci/mutate-script` already needs `verify/mutation-pass`.
2. **`mutate.sh` greps the test files for the marker itself.** One component, but a second parser of the marker's format in shell, and only this repository's command would know it (`reuse-first`).
3. **`verify` leaves every requirement whose tests carry a marker out of the plan.** Simpler, but it would leave unjudged the many requirements whose tests read files and also run their code (`crates/clm-check/tests/cli.rs`, `crates/clm-health/tests/command.rs`), which the pass judges well today. The marker only matters when the coverage run finds none of the component's code run, which only the command knows.
4. **Not-judged whenever the tests run some line of other code but none of the block's.** Needs no marker, but a test that calls only another component's code is exactly the misplaced test the pass should report.
5. **Add the marked files to the block's `code` lines.** They are Markdown, specs and other crates' test files: no mutation tool of Rust changes a comment, so nothing would be judged and the empty-test branch would still fire.

## Outcome

- `verify/mutation-pass`: each block lists, after its `test` lines and before `limit 20`, one `reads <glob>` line per distinct marker glob in the files holding its citing tests, sorted. Design names why.
- `ci/mutate-script`: a block holding a `reads` line whose tests run no line with a mutant of its files, or whose StrykerJS mutants are all `NoCoverage`, is `not-judged`; without a `reads` line it is `survived` as before.
- `check.affected`'s Contract names `readers` and `reads_in` as library calls; both exist and are proven by `check.affected/affected-readers`'s tests, so nothing is built there.
- The decision `mutation-after-land` records the step.

## Edge cases and workarounds

- An empty test in a file that carries a `clm:reads` marker reads as `not-judged`, not `unproven`. Accepted: the marker is a declaration a reviewer sees, and a file that reads no repository file has no reason to carry one.
- A `reads` line changes nothing for a block whose tests run its code: such a block is judged by its mutants as today.

## Constraints

- `verify` builds first: `mutate.sh` ignores an unknown plan line, so the plan can carry `reads` before the script reads it; `ci/mutate-script` needs `verify/mutation-pass` already.
- No new setting, no new plan line beyond `reads`, no new verdict word.
