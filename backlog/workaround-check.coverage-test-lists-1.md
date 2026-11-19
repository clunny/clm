---
title: "Stopped to raise a spec change: building it breaks land's test of the old rule"
kind: workaround
components: [check.coverage]
requirement: check.coverage/test-lists@bbdcfe2f
---

Building `check.coverage/test-lists` at `bbdcfe2f` makes `deleted_tests` return nothing for a test renamed in its own file that keeps its markers. `crates/clm-land/tests/deleted_tests.rs::a_renamed_test_refuses_the_land`, owned by `land`, uses exactly that fixture and expects the land refused, so the build would fail `land`'s test. This build may not edit `land`'s file, and `land/deleted-tests-approved` Needs `check.coverage/test-lists`, so `land` can't move its test first. The build stopped and filed `backlog/building-the-renamed-test-rule-breaks-land-s-test-of-the-old-rule` asking how to sequence the two.
