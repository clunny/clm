---
status: done
resolves: [a-cited-test-deleted-beside-an-uncited-test-of-its-name-that, a-test-that-loses-its-last-marker-beside-a-new-citing-test]
---

## Intent

Two more ways a branch can delete or strip a citing test without `deleted_tests` returning it close, so `clm land` asks for a `deleted-tests.txt` for them (`removal-needs-a-yes`): a cited test deleted beside an uncited test of its name that gains a marker (`backlog/a-cited-test-deleted-beside-an-uncited-test-of-its-name-that`), and a test that loses its last marker beside a new citing test, which the rename rule took for a rename (`backlog/a-test-that-loses-its-last-marker-beside-a-new-citing-test`). `check.coverage/test-lists` is reworded; the decision `renamed-test-keeps-its-citations` is amended to match.

## Problem

- `test-lists` counts a bare name's gone tests by how many tests of it hold a marker, base against head. Base `a.test.ts` holds two `test('two')`, the first citing `a/x`, the second with no marker; the head deletes the cited one and gives the other a marker. Each side holds one citing `two`, so nothing is taken and the deletion lands with no list. It is the same-name form of the hole `20261119-an-uncited-test-renamed` closed for renames; it is reachable where titles may repeat in a file (TS/JS).
- The requirement's summary says a test "that loses its last marker" is gone, but the rename rule lets one count as renamed. Base `tests/t.rs` holds `one` and `two`, each citing `a/x`; the head holds `one`, `two` with no marker, and a new `second` citing `a/x`. `two` is taken, `second` is new and fits it, the file has room for one rename, so nothing is returned. A test still in its file under its name was not renamed.
- The code does what the counting rule states, so each fix is a spec change (`code-fix-needs-a-written-rule`). Both items were filed today by the review of `check.coverage/test-lists@d258ae73`'s build; no change since 2026-11-12 (`20261119-a-test-that-gains`, `20261119-an-uncited-test-renamed`, `20261119-a-removed-requirement-s-tests-elsewhere`) and no code fix resolves either.

## Approaches

- **Taken, first item: a file's *gained* tests of a name don't count among the head's citing tests of it.** In each file, a name's unmarked tests that are fewer at the head, up to the number of its citing tests at the head, may have gained a marker, and stand for no citing test of the base. They are taken off the head's count before `n - m` is formed, and off the file's own count when it gives its tests. This is per file, so a citing test moved to another file under its name stays not gone, and it changes nothing where a file holds no unmarked test of the name. The trees can't tell an unmarked test deleted beside a kept cited one from a cited one deleted as its namesake gains a marker; the rule errs toward refusing, as every other unclear case of `test-lists` does, and the requirement says so.
- **Taken, second item: a name's renames in a file number no more than how many fewer tests of that name, marker or not, the file holds at the head.** A test whose name its file still carries as often as before wasn't renamed. It is a second cap beside the room cap on the existing pass. Counting tests of the name rather than asking whether the name is still carried at all keeps a rename of one of two same-titled tests a rename (scenario *one of two tests of a name renamed*).
- **Rejected: counting a name's tests in all, marked or not, across files (the item's own example).** A cited test moved to another file while an unmarked namesake is deleted from its file would be counted gone, and an unmarked test deleted from one file would count against a citing test of the same name in another. The per-file gained count takes the item's case without either.
- **Rejected: accept either gap.** Each lets a removal land without the human's yes, and each fix is one clause on a pass that already counts per file and name (`correctness over convenience`).

## Outcome

- `check.coverage/test-lists` (modified): defines a file's *gained* tests of a bare name and takes them off the head's citing count; caps a name's renames in a file at how many fewer tests of that name its head holds; states the ambiguous same-name case as counting the cited test gone. Three scenarios added: *a cited test deleted beside an uncited test of its name* (the first item's case, returned), *one of two tests of a name renamed* (nothing returned) and *a test that loses its last marker beside a new citing test* (the second item's case, returned).
- `docs/decisions/renamed-test-keeps-its-citations.md` states the rule as amended, and `cuts-are-what-a-program-lists` says what `clm land` refuses now covers a cited test an uncited namesake may have replaced.
- `docs/guide/how-it-works.md` tells sessions the two new refused cases and how to get past them (`method-in-the-plugin`).

## Constraints

- `deleted_tests`' signature and return type stay as they are; `clm land`, `clm land --check-only` and `clm coverage --deleted-tests` change only through what it returns.
- Every scenario of `test-lists` as it stands keeps its outcome: in none does a file lose an unmarked test of a name a base test cites while it holds a citing test of that name at the head (in *a test that gains its first marker* `quiet` gains, but no base test cites `quiet`), and in none is a taken test's name still carried by its file as often as at the base.

## Edge cases and workarounds

- A branch that deletes an unmarked test whose title a kept citing test of the same file shares is refused, though nothing cited was deleted: the trees can't tell it from the first item's case. It errs toward refusing; the build deletes it on a branch of its own or renames one of the two first, or the change lists it. Repeated titles in one file are rare, and mostly in TS/JS.
- A citing test moved under its name into a file that loses an unmarked test of that name while the moved one is its only citing test of the name there is counted gone: the gained count is per file and can't tell the move from the unmarked test gaining a marker. It errs toward refusing; the build moves it on a branch of its own.
- The same holds for an unmarked test renamed away from a title a kept citing test of its file shares: its loss counts as a gained marker.
- The doc comment on `deleted_tests` (`crates/clm-coverage/src/lists.rs`) follows the requirement.

## Open questions

None.
