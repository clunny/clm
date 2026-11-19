---
title: "A wait names a line of the pending list, and no reader takes a wait's component from its ID"
date: 2026-11-19
status: accepted
---

`a-removed-requirement-s-tests-elsewhere-are-listed` has an obsolete requirement pending on each component that owns a file citing it, and blocks its line on its own component by its own ID while another line is left. It said the queue reads that as it reads any pending requirement, so there was no new kind of blocker. The first build of it showed that to be false, and was reverted: the queue's priorities take a waited requirement's component from the first part of its ID, so the wait seeded the waiting component itself; the blockers view keys the pending list by ID, which folds a requirement's several lines into one; and the build prompt told a build that `b/gone` is blocked by `b/gone`.

**A line is one requirement on one component.** That has been so since `node-removal` put a removed component's requirement on each owner of a file that cites it. An ID names a line only while the requirement has one, which every requirement the spec holds does and an obsolete one may not.

**The own ID stays in `waits_on`, and one call resolves it** (`check.coverage/wait-lines`). `waits(lines, line)` gives, for each ID a line waits on, every line of that ID other than the line itself. For a need, a move's target or a serving rule that is the one line the ID has; for the own ID it is the requirement's lines on the other components. The queue's priorities, the blockers view and the build prompt read a wait's component from it. `clm pending` still prints `blocked b/gone`, and the queue's JSON is unchanged.

**A move's target on the line's own component stays a blocker while it is itself blocked.** The line dropped it whatever its state, so a build could be told to re-cite a test to a requirement its prompt had left out as blocked.

**The sequence** (`expand-contract`). The change to what `pending` returns is one another component's tests asserted, so it takes three steps, not the two first designed: `orchestrator.run` reads both placements, which its code has done since 2026-11-19; `check.coverage` switches the placement; and `orchestrator.run` then removes the path for the old one, marked `clm:remove-by` this decision's change, with the one test that proves only it, which the change lists. Three tests of `crates/clm-run/tests/run.rs` asserted the old placement through the real `pending`. A small code branch takes those assertions out before `check.coverage` builds: a requirement names no test (`requirements-state-what-the-contract-shows`), and the step states no behaviour. `check.coverage/pending-report` no longer needs `orchestrator.run/pending-prompt`, which now needs it.

**Why the first order failed.** `orchestrator.run` was built first, as designed, and did the reverse of its step: the step was written under "What the builds do" in `change.md`, which no build is given, and it asked the build to remove the old path while `pending` still gave the old placement, which would have had a prompt tell a build to delete a test the land refuses it. A step a build must take is stated in a requirement, or made by a session that knows it.

Considered:
- **A field of its own for the wait on other lines.** Rejected: it is a third kind of blocker in the queue's JSON, the web app and the panel, and `clm-run` builds `Pending` by struct literal, so the field is itself a change in steps. It would also leave the two readers that key lines by ID wrong for any requirement with several lines.
- **Each reader special-casing a wait on the line's own ID.** Rejected: three copies of one rule, and the next reader forgets it.
- **Removing the old path in `orchestrator.run`'s first build.** Rejected: between the two lands the prompt is wrong for the case, and the build that was asked to declined.
