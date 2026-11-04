---
title: "The queue names a deadlock between a backlog block and another change's waits"
date: 2026-10-22
status: superseded
superseded_by: requirement-blocks
---

A task of change `A` can be blocked by a backlog item that change `B` resolves, while one of `B`'s items waits for that very task, directly or through other items (cross-change waits, `waits_for`, `after`, or `finish` waiting for its tasks). Neither moves, and until now `clm next` just went quiet. `task-deletes-fixed-items` removes the usual cause, but a session can forget to delete the item, and no task may fix it alone.

So the queue derives the cycle each time it's read, as it derives the waits. It reports one only when every unfinished item of `B` (`A` itself, when it resolves its own block) depends on the blocked task. While any item of `B` is free, it may be the task that fixes and deletes the item, which is the normal flow, so warning then would be noise. It still reports the task as `blocked`, with the deadlock added. `clm next` prints a `deadlock:` line to stderr, and `clm claim` and `clm run` add it to their refusal, saying which item to delete once it's fixed. Whether to drop the block instead is the human's call. Neither changes its exit code or stdout, so `implement` loops are unaffected.

Alternatives considered:
- Failing `clm check`. The cycle comes from state already on `main`, so every PR would fail, including the one that breaks the cycle.
- Breaking the cycle automatically by ignoring the block. The block may be real: only a human or the fixing task knows whether the item is fixed.
