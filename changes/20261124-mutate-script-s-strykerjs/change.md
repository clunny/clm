---
status: applied
resolves: [mutate-script-s-strykerjs-fixture-runs-its-tests-whether-or-not-vitest-keeps-only-related-ones]
---

## Intent

`ci/mutate-script` states that StrykerJS runs each of a block's vitest tests whether or not it imports a file StrykerJS mutates, and its StrykerJS fixture proves the real tool does: beside the test that notices and the blind one, the fixture holds a test file that imports none of its source files and only reads a file of the repository, judged in a StrykerJS run of its own on the noticed function's lines, which must finish with every mutant `NoCoverage`. A StrykerJS that kept only the tests importing a mutated file would run no test there and fail, so the fixture fails before any block. This is plan work under `backlog/a-passing-check-proves-less-than-the-spec-claims`, as the earlier mutation-pass changes were.

## Problem

`dcee2c44` made `mutate.sh` set StrykerJS's `vitest.related` to false. Left true, vitest kept only the tests that import a mutated file: five `web` blocks whose tests read files ran no test, StrykerJS exited "No tests were executed", and every pass from `6d6325c6` to `88470edf` failed before most blocks started. That fix was code-only, against the clause "with the block's vitest `test` lines alone", which says which tests are given but not that each runs whatever it imports. Only a fake-`npx` test checks the setting, by the configuration's text. The real-tool fixture's noticing test imports the function, so it passes with `related` true too, and a pinned StrykerJS that renamed or ignored the option would bring the failure back unseen. The rule belongs in the requirement, and the fixture, the only place the real tool is judged, must notice its loss (`code-fix-needs-a-written-rule`, `proven-by-tests`).

## Approaches

- **Read the fixture's report for the import-free test.** Put the test file in the fixture's existing first run and require StrykerJS's JSON report to list it among the tests it ran. No extra run, but it leans on the report's `testFiles` listing tests that cover no mutant, a detail of the pinned version that a later version may drop, and it would show nothing about StrykerJS's exit when no test runs, which is the failure the passes met.
- **A run of its own with that test alone (chosen).** A third StrykerJS run per npm directory, with the import-free test file alone and the noticed function's lines. With the option honoured, vitest runs the test, it covers nothing, and StrykerJS reports every mutant `NoCoverage`; with it lost, StrykerJS runs no test and fails exactly as the passes did. It reuses the script's existing counting (`caught`, `missed`, `untested`) and costs one StrykerJS start per directory per pass, after the land (`mutation-after-land`), where time is not a land's.
- **Pin the option by the configuration's text.** What the fake test already does; it proves the script writes the option, not that the tool obeys it.

## Outcome

- `ci/mutate-script` says each of a block's vitest tests runs whether or not it imports a file StrykerJS mutates, so a block whose tests read files gets StrykerJS's `NoCoverage` verdict instead of a failed run.
- The StrykerJS fixture gains a second test file that imports none of its source files and reads a file of the repository, judged alone in a run of its own; the script goes on only when that run reports every mutant `NoCoverage`, and fails naming StrykerJS and the fixture when the run fails.
- Two scenarios: *the StrykerJS fixture runs a test that imports nothing* (the run's shape) and *a StrykerJS that runs only the tests that import a mutated file* (the fixture fails first). The scenarios' default fake `stryker run` now reports the fixture's three runs as the real tool should.

## Constraints

- The fixture stays written by the script and tracked nowhere, inside the npm directory being judged, with that directory's own vitest and TypeScript (`mutate-script`'s fixture rule).
- Nothing in the land check runs the real tools (`land-checks-stay-fast`); the fake-`npx` tests prove the plumbing, the fixture proves the tool.
- Edge cases known now: the import-free test runs in node's environment, as the noticing test does, and reads a file outside the npm directory by a path relative to its own file, so it also needs the sandbox's links; no workaround is expected.
- Settled by the builder check: the run of the import-free test reports at least one mutant, every one `NoCoverage` (a `CompileError` or an empty report fails it); it uses the noticing test's ranges (`fixture.ts:1-1`) and comes after the fixture's other two runs, so the existing fake cases fail on the run they describe. The fake `npx` of `ci_check.rs` must tell this run apart by the test files its vitest configuration includes, since it shares the noticing run's ranges, and two tests' StrykerJS run counts rise by one. No test is deleted.
- Assumed, not yet shown by a real run: StrykerJS 10.1.1 exits 0 with every mutant `NoCoverage` for a lone test that imports nothing, with `related` false. `dcee2c44`'s passes showed both halves on `web`'s file-reading blocks; the fixture's first pass confirms it.

## What the requirements no longer state

Nothing: no `SHALL` sentence is lost and no scenario's outcome changes. The closing sentence on the scenarios' fakes gains the default for the new fixture run.

## Open questions

None.
