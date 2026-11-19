---
title: "A subagent's pin moves when its first record is re-recorded"
kind: bug
components: [effort, ledger]
---

`effort/record-command` pins each `<session>/<agent>` to the branch of the fold's `effort` record of it with the earliest `at`. The fold keeps only the latest record per `effort_id` (`ledger/effort-records`), so the `at` compared is each branch's latest re-record, not when it was first recorded.

When two clones recorded one subagent for different branches (clone A on `impl/a` at t1, clone B on `impl/b` at t2), a later re-record of `impl/a` (its counts grew) moves its `at` to t3 > t2, and the pin moves to `impl/b`. The next run grows `impl/b`'s record with the same entries. The two records converge on the same counts and then stop growing, so the work is counted on both branches, which the Design already accepts for this two-clone case; but the pin is not "fixed once recorded" as the Design's intent says.

Fixing it needs a spec change: either the fold, or `effort`'s reading of it, keeps the first `at` an `effort_id` was recorded at, or the requirement says the pin follows the latest records. Found by the spec-auditor on the `impl/effort` build of `20261118-a-subagent-s-work`.
