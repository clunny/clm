---
title: "A removal marker after a non-ASCII letter skips its item with no liveness check"
kind: bug
components: [check.coverage, check.unreferenced]
---

`clm unreferenced` accepts a `clm:remove-by <change-id>` marker unless an ASCII letter, digit or `_` comes before it (`check.unreferenced/removal-marked`). The scan behind `check/remove-by-markers` refuses one after any Unicode alphanumeric character: `follows_word` in `crates/clm-coverage/src/coverage.rs` uses `char::is_alphanumeric`. So a marker such as `éclm:remove-by 20260101-x` before an item makes the detector skip the item, but `clm check` never checks whether that change is live, and the item can stay skipped after its removal is abandoned. This contradicts `check.unreferenced`'s Design ("every marker it accepts is one `check/remove-by-markers` checks"). Expected: the two scans agree on which character may come before a marker. Either `follows_word` checks ASCII only, as `removal-marked` states, or the spec makes both Unicode. Found by the spec audit of the `removal-marked` build.
