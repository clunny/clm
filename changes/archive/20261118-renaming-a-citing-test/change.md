---
status: done
resolves: [renaming-a-citing-test-reads-to-the-land-as-deleting-it-so-a]
---

## Intent

A test renamed in its own file that still cites every requirement it cited is not a deleted test, so a build can give a test a true name without a spec change (`backlog/renaming-a-citing-test-reads-to-the-land-as-deleting-it-so-a`). Recorded as `renamed-test-keeps-its-citations`, which amends `cuts-are-what-a-program-lists`.

## Problem

- `check.coverage/test-lists` counts a test as gone when the tree holds fewer citing tests of its name, so a rename is a deletion, and `land/deleted-tests-approved` refuses it unless an approved `deleted-tests.txt` names it.
- A code branch can't add to a `deleted-tests.txt`, so a build that must rename a test needs a whole spec branch. `cuts-are-what-a-program-lists` rejected excusing a rename because "one line on the list is its whole cost", which holds for a scenario on a spec branch and not for a test on a code branch.
- On `impl/host` a build changed an exit status and renamed two tests in `crates/clm-host/tests/command.rs` to say the new one; the land refused, and the build put the false names back with a comment explaining them. A rule that makes a test's name lie works against the reader the name is for.

## Approaches

- **Taken: match a gone test with a new name in its own file that cites at least what it cited.** One for one, the new name carried by no citing test at the base, IDs compared with versions left out. The check is about names already: a build may rewrite a test's body under its old name, keeping its markers, and the land has never refused it, so a new name with the same citations in the same file gives no more room than that. Whether the test proves what it cites stays the judge audit's and the mutation pass's.
- **Rejected: a rename the build records, such as a commit trailer.** The session's word, where the two trees already show the same thing to a program.
- **Rejected: matching a new name anywhere in the tree.** Any test elsewhere citing the same requirement would stand for a deleted one.
- **Rejected: comparing bodies.** The case met renamed the tests because what they assert changed, so it would never match.
- **Rejected: accepting it.** It leaves builds the choice between a false name and a spec branch per rename.

## Outcome

- `check.coverage/test-lists` (modified): a test `deleted_tests` would return counts as renamed, and isn't returned, when the head's text of its file holds a citing test whose bare name no citing test at the base carries and that cites every ID it cited; gone tests are matched in the order they are returned, each with the first unmatched such test of its file in line order. Its scenario *a renamed test* now returns nothing, and three scenarios are added: a rename that cites less, a rename into another file, and two gone tests with one new name. Its Contract line on `deleted_tests` says the same.
- `land/deleted-tests-approved` (modified): its refusal's advice names the new way through ("or under a new name in its file citing every requirement it cited"), its scenario *a renamed test* lands, and a scenario *a renamed test that drops its citation* is refused. `clm coverage --deleted-tests`, which prints what `deleted_tests` returns, follows with no change of its own.

## Constraints

- No test that is gone today because it was deleted, moved and renamed at once, or stripped of its last marker becomes not gone.
- `deleted_tests`' signature and its return type stay as they are; `clm land`, `clm land --check-only` and `clm coverage --deleted-tests` change only through what it returns.

## Edge cases and workarounds

- Greedy matching in returned order can miss a pairing that exists (a gone test citing `a/x` takes the only new test citing `a/x` and `a/y`, leaving a gone test citing both unmatched). It errs toward refusing, which the build answers by keeping a name; a full matching isn't worth its code for a case not met.
- `base_files` is what the caller passes: `clm land` passes only the files the branch touches, `clm coverage --deleted-tests` the whole tree. So a new name that only an untouched file's test carries at the base counts as new for the land and not for `clm coverage`. Accepted: the name is new in the file the rename happens in, which is what the land judges.
- A test that held no marker at the base and gains one counts as a new name, so deleting a citing test and putting its marker on an uncited test in the same file passes. Intended: it is the same as rewriting the gone test's body under its name.
- Until this is built, `clm land` runs `main`'s `clm`, which still refuses a rename, so the build renames no citing test of its own: it keeps `a_renamed_test_refuses_the_land` under its name, re-cited to *a renamed test that drops its citation*, where the name stays true, and adds a test for *a renamed test* landing. The fixture of `scenario_a_renamed_test` in `crates/clm-coverage/tests/test_lists.rs` becomes *a renamed test that cites less*, and the doc comment on `deleted_tests` follows the Contract.
- The two tests the item names (`page_open_exits_2_when_gh_fails`, `a_failing_gh_pr_view_exits_2_and_opens_nothing`) need no follow-up item: `20261118-a-failed-pull-request`, applied, has `host`'s build make that failure exit 2 again, which makes their names true; that build owns the comment beside the first.

## What the requirements no longer state

- `check.coverage/test-lists`: that a renamed test is gone (the clause "and a renamed test, a deleted one, and one that loses its last marker each are" now lists only a rename that also moves the test or cites less), and its scenario *a renamed test*'s outcome "it returns `tests/t.rs::two`, since a program can't tell a rename from a removal", now "it returns nothing".
- `land/deleted-tests-approved`: its scenario *a renamed test*'s outcome "it exits 1 naming `two` under `land/deleted-tests-approved`", now "it lands" for a rename that keeps its marker.
- `cuts-are-what-a-program-lists`: "A renamed test is: a build keeps a test's name."

## Open questions

- **This relaxes a check `removal-needs-a-yes` cites, and the gate doesn't see it.** The scenarios keep their names and no `SHALL` sentence is dropped, so `clm gate` prints `spec` and lists no cut; the architecture audit found that whether the human said yes to dropping the refusal of a rename rests on the session. This draft ran unattended, so the human hasn't: it is left for review rather than landed on checks.
- `plugin/skills/propose/SKILL.md` says the land refuses any renamed test; it is prompt text a spec branch can't edit, so `backlog/the-propose-skill-says-the-land-refuses-every-renamed-test` (`kind: deferred`, `plugin`) carries it. The guide pages `docs/spec-format.md` and `docs/guide/how-it-works.md` are reworded here.
