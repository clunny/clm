---
title: "The land and `clm coverage` can disagree about whether a test was renamed or deleted"
kind: bug
components: [land, check.coverage]
---

Step: building `check.coverage/test-lists` at `bbdcfe2f`, which counts a gone test as *renamed* when its file at the head holds a citing test of a bare name "no marker-holding test of `base_files` carries". The code review found this.

What happens: `clm_coverage::deleted_tests` judges which names are new from the `base_files` it is given. `clm coverage --deleted-tests` passes the whole tree (`crates/clm-check/src/lists.rs`), but `clm land` passes only the files the branch changed (`crates/clm-land/src/deleted_tests.rs`, `touched`). So a name a test in an untouched file already carries looks new to the land.

Example: at the base, untouched `tests/u.rs` holds a citing test `kept`. A branch deletes `two` (citing `a/x`) from `tests/t.rs` and adds a test `kept` citing `a/x` there. `clm coverage --deleted-tests` lists `tests/t.rs::two` as gone; `clm land` counts it as renamed and lands the deletion with no `deleted-tests.txt` naming it (`removal-needs-a-yes`).

Expected: the land and `clm coverage` reach the same verdict, for example by the land passing every base file's citing tests' names (or the whole base tree's tests) to `deleted_tests`, or `deleted_tests` taking the base's names apart from the files it compares. No requirement as written states which files the land passes, so this needs a spec change.
