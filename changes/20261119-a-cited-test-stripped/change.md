---
status: applied
resolves: [a-cited-test-stripped-beside-a-deleted-unmarked-namesake-and-a]
---

## Intent

One more way a branch can strip a citing test's last marker without `deleted_tests` returning it closes, so `clm land` asks for a `deleted-tests.txt` for it (`removal-needs-a-yes`): a cited test stripped of its marker beside a deleted unmarked test of its name and a new citing test (`backlog/a-cited-test-stripped-beside-a-deleted-unmarked-namesake-and-a`). `check.coverage/test-lists` is reworded; the decision `renamed-test-keeps-its-citations` and the guide's account of refused renames are amended to match.

## Problem

- Base `a.test.ts` holds `test('two')` citing `a/x`, then `test('two')` with no marker; the head holds one `test('two')` with no marker and a new `test('second')` citing `a/x`. The cited `two` is taken. The file's gone unmarked tests of `two` are counted against every test of `two` at the head, and the head's one unmarked `two` meets the base's one, so none is gone and the file has room for one rename; `two` is held one fewer time, so `second` fits and nothing is returned.
- The trees can't tell this from the cited `two` renamed to `second` beside a kept unmarked `two`, a true rename. Everywhere else `test-lists` errs toward refusing when the trees can't tell, and it states that a test losing its last marker is gone; here a test that may have lost its marker counts as renamed. It is the same-title form of the case `20261119-a-cited-test-deleted` closed (*a test that loses its last marker beside a new citing test*), reachable where titles may repeat in a file (TS/JS).
- The code does what `test-lists` states, so the fix is a spec change (`code-fix-needs-a-written-rule`). The item was filed today by the review of `check.coverage/test-lists@b02100ac`'s build, after `20261119-a-cited-test-deleted` was applied; no change since 2026-11-12 and no code fix resolves it (`20261119-a-removed-requirement-s-tests-elsewhere` and `20261119-an-obsolete-requirement-nobody` touch `check.coverage` for removed requirements, not `deleted_tests`).

## Approaches

- **Taken: a file's *stripped* tests of a name don't count among the head's unmarked tests of it when its gone unmarked tests are counted.** The mirror of the *gained* count `20261119-a-cited-test-deleted` added: in each file, a name's citing tests that are fewer at the head, up to the number of its unmarked tests at the head, may be cited tests that lost their marker, and stand for no unmarked test of the base. A file's gone unmarked tests of a name become its base's unmarked tests of the name less what the head accounts for: its unmarked tests of the name less its stripped ones, plus its gained ones. The item's case then has one gone unmarked `two`, which takes `second`, the file's one new test, so the cited `two` has no room and is returned.
- **Rejected: count against the head's tests of the name, marked or not, less the stripped ones (the closest reading of the item's own example).** Base `two` cited twice and unmarked once, head `two` cited once and unmarked once beside a new citing `second`: the kept cited `two` would account for the base's unmarked one, leaving room for `second`, though the trees can't tell a stripped `two` beside a deleted unmarked one from a rename. Counting the head's cited tests only as far as they gained a marker refuses it too.
- **Rejected: accept the case.** It lets a removal land without the human's yes, and the fix is one clause on a count the requirement already makes per file and name (correctness over convenience).

## Outcome

- `check.coverage/test-lists` (modified): defines a file's *stripped* tests of a bare name, counts its gone unmarked tests against its head's unmarked tests less its stripped ones plus its gained ones, and states the ambiguous same-name case as counting the cited test gone. Two scenarios added: *a cited test stripped beside a deleted uncited test of its name* (the item's case, returned) and *a cited test stripped beside a kept citing namesake* (the rejected approach's case, returned, so a kept cited test doesn't account for an unmarked one).
- `docs/decisions/renamed-test-keeps-its-citations.md` states the rule as amended, and `docs/guide/how-it-works.md` names the new refused case (`method-in-the-plugin`).

## Constraints

- `deleted_tests`' signature and return type stay as they are; `clm land`, `clm land --check-only` and `clm coverage --deleted-tests` change only through what it returns.
- Where a file has no stripped test of a name, its gone unmarked tests of the name are what they were: its base's unmarked tests less its head's unmarked ones less `min(that, its head's cited ones)` equals its base's unmarked tests less all its head's tests of the name, none where it is no more. Every scenario of `test-lists` as it stands keeps its outcome: in each where a name has stripped tests (*a test that loses its marker*, *a test that loses its last marker beside a new citing test*) the base holds no unmarked test of that name.

## Edge cases and workarounds

- A branch that truly renames a cited test beside a kept unmarked test of its title in the same file is refused: the trees can't tell it from the item's case. It errs toward refusing; the build renames on a branch of its own after renaming or deleting the unmarked namesake, keeps the name, or the change lists it. Repeated titles in one file are rare, and mostly in TS/JS.
- The doc comment on `deleted_tests` (`crates/clm-coverage/src/lists.rs`) follows the requirement.

## Open questions

None.
