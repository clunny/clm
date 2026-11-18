---
status: applied
resolves: [a-deleted-runs-task-ids-are-forgotten]
---

## Intent

The build loop keeps the task IDs of every run of a name it has finished with, however the run left its list. They are kept by run name on the loop file, not on the run, so a run that `clm build start`, `clm build release` or a step drops still leaves its task IDs behind: a later run of the same name then refuses the old task at `clm build launched`, and a late `clm build returned <name> --task <old-task>` changes nothing, also when no run of the name is listed. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether the exception made for the two rules covers it.

## Problem

`launched-refuses-an-earlier-task` keeps a run's earlier task IDs with the run of that name, and only when the loop clears a run's task ID or replaces a listed run with a new one of its name. Three paths remove a run outright and keep nothing: `clm build start` drops every `returned` run, `clm build release` forgets the released build's run, and a step drops a fold or archive orphan (and a run whose relaunch was refused) for the plan to compute again. When the plan later starts a run of the same name, the new run has no earlier IDs, so `clm build launched <name> <old-task>` is accepted and a late `clm build returned <name> --task <old-task>` is recorded for the new run. The common case: `build:a` lands with task `t-1`, a new session runs `clm build start`, and `build:a` is launched again. No requirement states what a deletion keeps, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`).

The design is wrong, not only the code: keeping the IDs on the run ties them to a record the loop deletes on several paths, and each new path that deletes a run would have to remember to hand them on.

Nothing since 2026-11-10 fixes it: `20261117-clm-build-step-prints` and `20261117-a-late-report-of` introduced the two rules as they stand, `20261116-the-build-loop-shows` and `20261117-the-build-loop-drafts` don't touch task IDs, and no commit with a `Resolves: backlog/` trailer since then touches `crates/clm-loop`.

## Approaches

- **Taken: the loop file keeps the IDs by run name, apart from its runs.** One map from a run's name to the task IDs of the runs of that name the loop has finished with. Every path that clears a task ID or removes a run adds the run's ID under its name, and the two rules look it up by name, so they hold whatever path the earlier run left by, also when no run of the name is listed. It replaces the per-run list rather than adding a second home beside it, and the replacement's hand-on of IDs from the old run to the new one goes (`redesign-before-workaround`).
- **Not taken: keeping a stub run for each deleted run.** Every reader of the run list (the plan, the queue, the places, the summary, the housekeeping) would have to skip stubs, a rule repeated across the crate for a list of IDs.
- **Not taken: a second, loop-level list only for deleted runs, beside the per-run one.** Smaller to build, but the same fact would live in two places and every lookup would read both.
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. The case it leaves open, a new session after a land, is the one every resumed loop goes through, and the human made the two rules an exception because every remaining phase is built through that loop; whether this is part of that exception is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/launched-refuses-an-earlier-task` (modified): the loop file keeps finished runs' task IDs by run name, whatever path removed the run, a loop file an earlier `clm` wrote has the IDs it kept with a run kept under that run's name, and no step removes one. New scenarios: a landed build dropped by a new session's `clm build start`, and a released build's task.
- `orchestrator.loop/returned-ignores-an-earlier-task` (modified): a return from a task kept under its name is ignored also when no run of the name is listed, where `return-rules` would refuse it. New scenario for that case.
- `backlog/the-fix-for-a-deleted-run-s-task-ids-is-outside-the-freeze` (`kind: question`) blocks the two requirements until the human answers.
- No capability test sees a difference: `capability/build-from-spec` already reports a distinct task ID for each run.

## Constraints

- The loop file's `version` stays 1; the new field is optional, and the per-run list an earlier `clm` wrote is read, not written.
- The commands' output and exit codes are unchanged but for the one case of `returned` with no run listed, which exits 0 where it exited 1.

## Edge cases

- **The kept IDs grow for the life of the loop file**, one per finished run, since a late report may come any time while its session lives. A task ID is a short string; at hundreds of runs the file stays small. Accepted.
- **A loop file renamed `loop.damaged`** starts afresh with no kept IDs, as with every other state it held.

## Open questions

None.
