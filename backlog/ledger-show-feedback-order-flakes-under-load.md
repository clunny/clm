---
title: "Ledger show tests flake when the wall clock steps back"
kind: bug
components: [ledger]
---

Step: `clm land --check-only` on `impl/config`, in `.github/scripts/ci-check.sh`'s test run, on WSL2, beside other builds' tests.

What happened: two runs of the check failed, each in a different `clm-ledger::ledger` test that expects `clm ledger show` to list records in the order they were appended:
- `show_prints_feedback_on_one_line` (`req:ledger/feedback-records`): the record appended last got `at` `15:30:16.459Z`, earlier than the first record's `15:30:16.570Z`.
- `show_effort_and_decided`: the `decided` record, appended after the `effort` record and a 5 ms `tick()`, got `at` `15:31:49.248Z`, while the `effort` record got `15:31:49.969Z`.

`at` comes from `SystemTime::now()` (`crates/clm-ledger/src/append.rs`, `now()`), and the wall clock on this host can step backwards by hundreds of milliseconds (WSL2's clock sync). The tests passed every time they ran alone (5 of 5 for the first).

Expected: these tests don't depend on wall-clock time moving forward between two appends. For example, they could compare against the order `show` defines for equal or reversed `at` values, or set the records' times explicitly. Whether `show`'s time order should also hold when the clock steps back is a question for the ledger's spec.
