---
title: "clm-land's lock priority tests fail under a loaded host and refuse an unrelated land"
kind: bug
components: [land]
---

`clm land impl/apply` (2026-11-19) failed its check script on two tests in `crates/clm-land/src/lock.rs`, `lock::tests::below_p1_the_priority_lane_goes_first_then_the_highest_priority` (assertion at lock.rs:575: left "ord2", right "pri3") and `lock::tests::priority_lane_runs_go_first_but_cannot_starve_the_rest`, with `SendError` panics from the helper threads at lock.rs:548. The branch touches only `crates/clm-apply` and two backlog deletions. Both tests pass when rerun alone (`cargo nextest run -p clm-land --lib lock::tests`, 10 passed). They order waiters by timing, so on a host running other builds' tests at once (`tests-run-concurrently`) the order they assert doesn't hold, and an unrelated land is refused. Expected: the tests order the waiters by an event rather than by elapsed time, so load can't change the outcome.
