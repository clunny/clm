---
title: "A cited test deleted beside an uncited test of its name that gains a marker is not gone"
kind: bug
components: [check.coverage]
---

Found by the code review of the build of `check.coverage/test-lists@d258ae73`.

`test-lists` counts gone tests by how many tests of a bare name hold a marker. Base `a.test.ts` holds two `test('two')`, the first citing `a/x`, the second with no marker. The head deletes the cited one and adds a marker to the other. Base and head each hold one citing `two`, so nothing is taken and `deleted_tests` returns nothing: the deletion lands without a `deleted-tests.txt` (`removal-needs-a-yes`). It is the same-name form of the hole `20261119-an-uncited-test-renamed` closed for renames, and mostly reachable where titles may repeat (TS/JS).

The code does what the requirement states, so the fix is a spec change (`code-fix-needs-a-written-rule`): for instance, counting a name's tests gone when the head holds fewer tests of that name in total, marked or not, than the base did, or stating the gap as accepted.
