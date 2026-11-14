---
title: "A marker counts only inside a test, read from the file's text, and no test runner's output is read"
date: 2026-11-14
status: accepted
---

A requirement read as built when a `req:` string with the right version sat anywhere outside Markdown: a comment in production code, a constant, a file header. Nothing tied the string to a test.

Now a marker counts only on the lines of a test or a named group of tests, the comment directly above one included. `clm-coverage` finds those lines by the file's extension, for the three language families `check/skipped-tests` already reads: a Rust function with a test attribute; a call of `it`, `test`, `describe` or `test.describe` in TypeScript and JavaScript; a `test` function or `Test` class in Python. It uses a line scanner, not a parser: the crate builds for WebAssembly for the Worker, which a parser generator's C runtime doesn't, and the rule needs only where a test starts and ends. The scanner is approximate, and a span it gets wrong shows as an `uncounted` marker. In a file of any other extension every marker still counts: CLM can't tell a test there, and refusing those markers would leave every requirement of a project in that language unbuilt. A language is added by a change.

On 2026-11-14 all 4,202 Rust markers of this repository sat in or directly above a test. Of 230 TypeScript markers, 16 were file-header comments, which left five requirements with no marker that counts. `clm coverage` lists such markers as `uncounted`, and the builds move them.

**Whether the test passed is not read from a runner.** The plan asked that a marker count only when its test passed in the check the build ran, and since `20261113-land-starvation` a land's verdict can be kept from an earlier `main` or composed from an overlap re-check. But a land needs a `checked` record for the merged tree's code key, the key covers test files, and every selection a check runs includes every file the branch changed (`check.affected/affected-files`). So the test holding a new or re-cited marker ran in a check that passed, whichever kind of verdict the land carried, and no new record is needed. Two gaps remain: a test the project's check never runs although it was selected, and a project with no `[land] check`. The mutation pass shows the first after the land, since a test that never runs notices nothing (`mutation-after-land`).

Considered:
- **Reading the runner's results**, as JUnit XML written to a place CLM names, and recording per test that it passed. It closes the first gap at the land. It needs a results format every adopter's check script must write, a join from a runner's test names to the tests CLM finds in the text, which differs per runner, and a ledger read in every place that computes coverage from file text alone: the CLI, `clm view --at` and the Worker. Rejected for now; taken up if the mutation pass shows tests that never ran.
- **Counting a marker of an unknown language only in a file a `tests` glob matches.** `markers_in` reads one file's path and text and knows no graph, which is what lets the Worker count what the CLI counts. Rejected.
- **Failing `clm check` on a marker outside a test.** A project that was green would turn red on upgrading. The requirement reads `unbuilt` instead, which is pending work, and the marker is listed.
