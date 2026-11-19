---
title: "A test that gains its first marker can hide a deleted test as a rename"
kind: bug
components: [check.coverage]
---

Step: building `check.coverage/test-lists` at `bbdcfe2f`. The code review found this.

What happens: `deleted_tests` counts a gone test as *renamed* when its file at the head holds a citing test whose bare name "no marker-holding test of `base_files` carries" and that cites at least what the gone test cited. A test that existed at the base with no marker carries no marker-holding name, so once it gains a marker it qualifies.

Example: at the base `tests/t.rs` holds `two` (citing `a/x`) and `quiet` (no marker). A branch deletes `two` and adds a marker citing `a/x` to `quiet`. `deleted_tests` returns nothing, so `clm land` lets the deletion through with no `deleted-tests.txt` naming it (`removal-needs-a-yes`).

Expected: only a name no base test carries, cited or not, counts as new. The code follows the requirement's text as written ("marker-holding test"), so the fix needs `test-lists` reworded first.

Two smaller points from the same review, both following the text as written: the matching is greedy in line order, so renames `A (a/x)`, `B (a/x, a/y)` to `F1 (a/x, a/y)`, `F2 (a/x)` leave `B` reported gone although both kept their citations; and when one test is deleted and a later one renamed, the earlier, deleted one takes the new name and the renamed one is reported. A spec change for the main point could settle these too (match the gone tests with the most IDs first, or a full bipartite match).
