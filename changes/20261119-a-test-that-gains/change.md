---
status: applied
resolves: [a-test-that-gains-its-first-marker-can-hide-a-deleted-test]
---

## Intent

A test the base held with no marker can no longer stand for a deleted test by gaining one, and the pairing of gone tests with new names renames as many as can be paired, so the order of a file's tests never turns a rename into a refusal (`backlog/a-test-that-gains-its-first-marker-can-hide-a-deleted-test`). `check.coverage/test-lists` is reworded; the decision `renamed-test-keeps-its-citations` is amended to match.

## Problem

- `deleted_tests` counts a gone test as renamed when its file at the head holds a citing test whose bare name "no marker-holding test of `base_files` carries". A test that sat at the base with no marker carries no marker-holding name, so once a branch gives it a marker it stands for any gone test citing no more than it now does. At the base `tests/t.rs` holds `two` (citing `a/x`) and `quiet` (no marker); a branch deletes `two` and cites `a/x` in `quiet`; `deleted_tests` returns nothing, and `clm land` lets the deletion through with no list naming it (`removal-needs-a-yes`).
- The pairing is greedy in line order: gone `A (a/x)` and `B (a/x, a/y)` renamed to `F1 (a/x, a/y)` and `F2 (a/x)` pairs `A` with `F1` and leaves `B` reported gone, although both kept their citations. A code branch can't add a `deleted-tests.txt`, so that false refusal costs a spec branch.

## Approaches

- **Taken: a new name is one no base test carries, cited or not.** The rename rule exists for a test a build gives a truer name; a test that already existed under its name is no rename of anything. Looking across every base file, not only the gone test's, also stops an unmarked test moved in from another file standing in.
- **Taken: pair as many as can be paired.** Taking the gone tests in the order `deleted_tests` returns them, each counts as renamed when it and every earlier one so counted can each be paired with a different head test it fits. That is a largest matching, made deterministic by the order: the sets of gone tests that can be paired at once form a transversal matroid, so this greedy pass in order reaches the largest number. The build is a few lines of augmenting-path search over a file's handful of tests.
- **Rejected: pair the gone tests with the most IDs first.** Mends the example but not every case (two gone tests of equal size each fitting a different subset of new names), where the full matching costs no more to state or build.
- **Accepted, not fixed: which of two equal tests is reported.** When one gone test was deleted and another renamed, and the new name fits both, the trees hold nothing that says which was which. The count is right (one is gone) and the earlier is counted renamed, so the later is the one a list must name; the requirement says so rather than leave it to the code.

## Outcome

- `check.coverage/test-lists` defines a head test a gone test *fits* (same file, holds a marker, bare name carried by no base test with a marker or without, cites every ID the gone test cited) and renames, in `deleted_tests`'s order, each gone test that can join a one-for-one pairing with those already renamed.
- Two scenarios added: *a test that gains its first marker* (reported gone) and *two renames that each keep their citations* (nothing returned).
- `docs/decisions/renamed-test-keeps-its-citations.md` states the rule as amended.

## Constraints

- `land/deleted-tests-approved` is unchanged: its message's "under a new name in its file citing every requirement it cited" still reads right, a new name being one the base never held.
- Edge cases known: a test renamed to a name an unrelated test in another base file carries is now reported gone (errs toward refusing; the build picks another name or the change lists it).
- Edge case known: a test renamed to the name of an unmarked base test that the same branch deletes is reported gone too, for the same reason.
- Scope at the land: `clm land` passes `deleted_tests` only the files the branch touches, so "no base test carries" looks across those; a move from another file touches both, so the unmarked-test-moved-in case is still caught.
- `docs/guide/how-it-works.md`'s sentence on renaming a test now says the new name must be one no test held before (`method-in-the-plugin`, from the architecture audit).
