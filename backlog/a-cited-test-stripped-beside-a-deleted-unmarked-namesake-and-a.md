---
title: "A cited test stripped beside a deleted unmarked namesake and a new citing test counts as renamed"
kind: bug
components: [check.coverage]
---

Found by the code review of the build of `check.coverage/test-lists@b02100ac`.

Base `a.test.ts` holds `test('two')` citing `a/x`, then `test('two')` with no marker. The head holds one `test('two')` with no marker and a new `test('second')` citing `a/x`. Gained is min(1-1, 0) = 0, so the cited `two` is taken; the file's gone unmarked tests are 1 - 1 (head total) = 0, so it has room for one rename, and `two` is held one fewer time (2 - 1 = 1), so `second` fits and `deleted_tests` returns nothing. The trees can't tell this from the cited `two` renamed to `second` beside a kept unmarked `two`, which is a true rename; elsewhere `test-lists` errs toward refusing when the trees can't tell (the cited one counts as gone). It is the same-title form of `a-test-that-loses-its-last-marker-beside-a-new-citing-test`, reachable where titles may repeat (TS/JS).

The code does what `test-lists` states, so the fix is a spec change (`code-fix-needs-a-written-rule`): for instance, count a file's gone unmarked tests of a name against its unmarked tests at the head less the cited tests of that name it lost a marker from, or state the case as accepted.
