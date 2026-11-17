---
title: "Tests assume the wall clock only moves forward"
kind: bug
components: [ledger, land]
---

Step: `clm land --check-only` on `impl/config`, in `.github/scripts/ci-check.sh`'s test run, on WSL2, beside other builds' tests. Three runs of the check failed in a row, each in a different test, and each because the host's wall clock stepped backwards by hundreds of milliseconds between two readings (likely WSL2's clock sync):

- `clm-ledger::ledger show_prints_feedback_on_one_line` (`req:ledger/feedback-records`): the record appended last got `at` `15:30:16.459Z`, earlier than the first record's `15:30:16.570Z`, so `clm ledger show` listed it first.
- `clm-ledger::ledger show_effort_and_decided`: the `decided` record, appended after the `effort` record and a 5 ms `tick()`, got `at` `15:31:49.248Z`, while the `effort` record got `15:31:49.969Z`.
- `clm-land::push_retry refused_every_time_then_rerun` (`req:land/land-push-retry`): at `crates/clm-land/tests/push_retry.rs:71`, `times[1] - times[0]` over the hook's logged push times panicked with "attempt to subtract with overflow". It failed 1 of 3 times when run alone.

The ledger's `at` comes from `SystemTime::now()` (`crates/clm-ledger/src/append.rs`, `now()`), and the push hook logs wall-clock times.

Expected: no test depends on the wall clock moving forward between two readings. For example, the ledger tests could set the records' times explicitly or not rely on `at` order, and the retry test could measure its backoff with a monotonic clock or use `saturating_sub`/`checked_sub` and a bound that tolerates a step. Whether `show`'s time order should hold when the clock steps back is a question for the ledger's spec.
