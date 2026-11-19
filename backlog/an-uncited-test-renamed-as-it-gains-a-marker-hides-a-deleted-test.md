---
title: "An uncited test renamed as it gains its first marker hides a deleted test"
kind: bug
components: [check.coverage]
---

Found by the code review of the build of `check.coverage/test-lists@1f461b0a`.

`test-lists` now makes a head test's name old when any base test carries it, cited or not, so a base test that keeps its name and gains a marker no longer hides a deleted test. But one that is renamed at the same time still does: its new name no base test carries, so it *fits*.

Base `tests/t.rs` holds `one` and `two`, each citing `a/x`, and `quiet`, with no marker. The branch deletes `two` and renames `quiet` to `quiet_now`, adding a marker citing `a/x`. `deleted_tests` returns nothing, and `clm land` accepts the deletion though no `deleted-tests.txt` names it.

The code does what the requirement states, so closing this needs a spec change: for instance, counting an uncited base test that is gone from its file at the head as a possible source of each fresh name there, so a fresh test stands for it before a deleted cited one; or stating the gap as accepted.
