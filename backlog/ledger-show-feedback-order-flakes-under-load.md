---
title: "Ledger show's feedback order flakes under load"
kind: bug
components: [ledger]
---

Step: `clm land --check-only` on `impl/config`, in `.github/scripts/ci-check.sh`'s test run, beside other builds' tests.

What happened: `clm-ledger::ledger show_prints_feedback_on_one_line` (`req:ledger/feedback-records`) failed: `clm ledger show` printed the third record (`commented ... n/r ... ok`) first, and its `at` read `15:30:16.459Z`, earlier than the first record's `15:30:16.570Z`, though it was appended last, after two `tick()`s of 5 ms. Run alone it passes every time (5 of 5).

Expected: the test's records carry `at` times in the order they were appended, or the test doesn't depend on wall-clock order under load. A 5 ms `tick()` and a timestamp from the clock don't guarantee it when the host is busy, and something gave the last record an earlier time than the first. Likely touches `tests-run-concurrently`.
