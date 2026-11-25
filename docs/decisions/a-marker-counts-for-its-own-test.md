---
title: "A marker counts only for the test whose lines hold it, never for a group's tests"
date: 2026-11-24
status: accepted
---

`markers-count-in-tests` let a marker count on the lines of a test or of a named group of tests (a `describe` block, a `Test` class), the comment directly above one included. That read one marker two ways. The coverage counted a marker above a `describe` as one citation of its requirement, while `clm health`'s `uncited-test` and the land's `deleted_tests` read a test's own lines and found the tests inside the block citing nothing: for `web`, 112 of its 119 `uncited-test` rows were such tests, and any one of them could be deleted with no `deleted-tests.txt` line though it was the block's proof (`backlog/a-marker-above-a-describe-block-counts-once-for-coverage-while`).

Now a marker counts only on the lines of a test, the comment and attribute lines directly above it included. A group is still read, to bound the tests inside it, but a marker on its own lines, the comment above its call included, counts for none of them: it is listed as `uncounted`, as a marker in a file header is, and the requirement reads `unbuilt` unless a test's own marker cites it. The coverage, `uncited_tests` and `deleted_tests` read a test's citations from the same lines, so a test is cited exactly when its own lines cite something, `clm health` says so, and its deletion needs a list line.

The rest of `markers-count-in-tests` stands as it was: a marker counts only inside a test in the three language families `check/skipped-tests` reads, found by a line scanner and not a parser, so the crate builds for WebAssembly; in a file of any other extension every marker counts; and whether the test passed is read from no runner, since every file a branch changes is in the selection of the check behind its land's verdict, with the mutation pass showing a test that never ran.

Considered:
- **A group's marker cites every test inside it.** The coverage would keep counting the block, and `uncited_tests` and `deleted_tests` would give each test the markers of the groups around it. It keeps today's 68 markers above blocks in `web/` counting, but `deleted_tests`' rule, already the longest in `check.coverage`, would read inherited markers in every count it makes (gained, stripped, renamed), one marker could stand for many tests whose deletions each need a list line naming them though no line of theirs names the requirement, and the mutation pass would judge a block's marker by the block. Rejected: one marker per test is what every reader already reads, and wave 6 had the builds of `web`, `web.views` and `web.client-live` move their markers onto each test anyway.
- **Failing `clm check` on a marker above a group.** A project that was green would turn red on upgrading, as `markers-count-in-tests` found for a marker outside a test. The marker is listed as `uncounted` instead, and its requirement reads `unbuilt`, which is pending work a build takes.
