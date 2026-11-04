---
title: "A skipped test in a file that cites requirements fails `clm check`"
date: 2026-10-27
status: accepted
---

A marker counts once the file exists and the suite passes (`proven-by-tests`), so a test marked `#[ignore]`, `it.skip` or `@pytest.mark.skip` still made its requirement current while proving nothing (`backlog/skipped-tests`). The item proposed reading the test runner's reports (JUnit XML) and mapping each skipped test to its markers.

Now `check/skipped-tests` reports any skipped test, by a fixed pattern per language, in a file that holds a `req:` marker. A test that is skipped belongs in a file that cites nothing; a test that cites a requirement runs.

Why not the runner's reports: `cargo test` writes no JUnit without an unstable flag or another tool, each language's runner reports differently, and `clm check` runs on trees where no suite ran (`clm sync`, the Worker). A marker would then count or not depending on whether a report was lying around, which isn't deterministic.

Why the whole file, not the marker's own test: tying a marker to the test it sits in needs a parser per language, and markers are already read per file (`tests-count-where-cited`, `fixes-count-where-cited`). The cost is that an unrelated skipped test must move out of a file with markers, a small, visible edit. Fixtures that hold a skip pattern are built from pieces, as fixtures holding markers are.
