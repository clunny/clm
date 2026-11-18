---
title: "clm-effort's more_work_appends_again fails under load, folding the first effort record instead of the second"
kind: bug
components: [effort]
---

Step: `clm land --check-only` on `impl/web`, with other builds' checks running on the host (`tests-run-concurrently`).

What happens: `crates/clm-effort/tests/record.rs`'s `more_work_appends_again` failed once at its last assertion. It got `left: (0, 10)` against `right: (30, 20)`: the fold held the first `effort` record for `s1`/`impl/a`, not the one the second `record` appended. Run alone three times right after, it passed each time.

Likely cause: the two appends happen within the same second. When the fold picks the record for an `effort_id`, it may break the tie by something other than append order. Either the fold should keep the later append for an `id` whatever its timestamp, or the test should not depend on the two appends falling in different seconds. Which is the bug is `effort`'s and `ledger`'s to judge.
