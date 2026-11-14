---
title: "After a land, a project's mutation command says whether new tests notice a change to the code, and a requirement whose tests notice none reads as not built"
date: 2026-11-14
status: accepted
---

A marker in a test that passed says the test ran. It doesn't say the test would fail if the code were wrong: CLM's own capability test for `prove-by-tests` got `current` from an empty test function. Builds were told to break each key rule by hand and see a test fail, which nothing recorded.

`clm verify` now runs a mutation pass after a check that passed, when the project sets `[verify] mutate`. CLM names no tool, as for `[land] check`: the command reads a plan and writes one verdict per requirement, `killed`, `survived <n>` or `not-judged`. Here it is a script that runs `cargo-mutants` and StrykerJS. The pass appends a `mutated` ledger record, and for each survivor lands a `bug` item naming the requirement under `unproven`; coverage reads a requirement so named, at its current version, as `unbuilt`, until a build deletes the item: by mending the test, which the next pass then judges as a changed test, or by finding it sound, which settles it, since an unchanged test is not judged twice.

**Which requirements.** One whose citing test was added, or changed in more than its markers, since the last pass. The plan said "changed or re-cited", and mutating "the changed code of its component". A requirement that was only re-cited has no changed test and often no changed code, and its tests are the ones an earlier pass saw. Judging it against a sample of its component's mutants misjudges sound tests: on 2026-11-14, 17 mutants of `clm-coverage`, run against every test citing it, were noticed by the tests of seven of its ten requirements in 18% to 88% of cases and by those of three in 0 to 6%, among them a test that only builds the crate for another target. So the changed code carries the verdict, the 20 further mutants from the rest of the component are a second chance for a new test whose requirement needed no new code, and a re-cite alone is not judged.

**What it doesn't reach.** Tests written before the pass (`backlog/tests-written-before-the-mutation-pass-were-never-judged`), requirements proven only by browser tests, which the script reports `not-judged`, and a new sound test of code the lands didn't change that 20 mutants miss, which costs one build's look.

**Why the verdict is a backlog item.** Verify pushes nothing to `main`, and coverage is computed from the tree alone in the CLI, in `clm view --at` and in the Worker. An item is a file in the tree, so every reader sees the verdict with no ledger, and adding an item lands on checks. Verify lands it itself through `clm land`; the route a red slow test takes, waiting for a person to take it in, would have a person judge a test.

**Cost.** Nothing at a land: the pass runs after the `verified` record is written and outside the check lock, with a `CLM_SLOT` of its own, one past the build places, so its tests may overlap a land's check as a build's do (`tests-run-concurrently`). Measured here, a mutant costs about 1 second with a crate's own tests and 10 to 18 with the root's CLI tests; a typical build's diff holds 5 to 15 mutants; a pass over 20 lands is 30 to 60 minutes of one core.

Rejected:
- **CLM calling `cargo-mutants` itself.** CLM knows no language's tools (`reuse-first` is the project's to apply to its own language).
- **Coverage reading the ledger's `mutated` records.** Every caller of the coverage would have to fold the ledger, the Worker included.
- **A new state word, `unproven`.** Every view matches the states exhaustively; `unbuilt` already means "not counted as built".
