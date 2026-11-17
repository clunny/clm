---
status: applied
resolves: [a-late-report-of-a-landed-build-s-subagent-is-recorded-for-the-next-build-of-its]
---

## Intent

A return from a subagent of a run the loop has already finished changes nothing of the run now listed under its name. `clm build returned` takes `--task <task-id>`, the ID of the task that returned; when that ID is one the loop keeps as an earlier run's of the name, the return is ignored, whatever its line and cause, and stderr says so. A late `none --cause context` of a landed build's subagent no longer finishes and rebuilds the component's next build, and a late `none --cause work` no longer counts a stop against it. It reads the task IDs `launched-refuses-an-earlier-task` keeps, so the question that holds that requirement back for the plan's freeze now holds both, and the `build` skill's sentence that passes `--task` joins the deferred item that already waits on that requirement.

## Problem

When a build lands before its subagent reports, `claim-gone-ends-a-build` finishes the run, and the plan may launch the component again under the same name, `build:<component>`. `clm build returned` names a run only by its name. `return-rules` ignores the old subagent's late `landed <sha>`, by the commit's time, but records any other line from it for the new run: `none --cause context` finishes the new build and rebuilds it, which puts a second subagent on a branch the first is still building, and `none --cause work` counts a stop against it, bringing its escalation one stop closer. No requirement states that a late return changes nothing beyond the `landed` case, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`).

Nothing since 2026-11-10 fixes it: `20261117-clm-build-step-prints`, which filed it, stops the wrong record at `launched` only, and no commit with a `Resolves: backlog/` trailer since then touches `returned`.

## Approaches

- **Taken: the session names the task a return came from, and the loop ignores a return from an earlier run's task.** The agent harness gives the session the task ID with every return, and `launched-refuses-an-earlier-task` already keeps each name's earlier task IDs, so the check is a lookup in what the loop file will hold (`reuse-first`). It is a check in `clm` with a test (`build-loop-in-clm`, `deterministic-enforcement`), and it covers every line and cause, and drafts as well as builds.
- **Not taken: extending the commit-time rule to other lines.** Only a `landed` line carries a commit; a `none` line carries nothing the loop can date.
- **Not taken: naming each run uniquely (`build:a#2`).** Every command, the prompt files (`loop run <name>`), the panel and the queue read the name; a new form of name changes them all for one case (`yagni`), as `20261117-clm-build-step-prints` already judged.
- **Not taken: making `--task` required.** A breaking Contract change for the skill and every session running the old text, which would need expand-then-contract (`expand-contract`); an optional flag keeps a session that doesn't pass it where it is now, and the skill's text is what makes sessions pass it.
- **Not taken: refusing a task ID that belongs to a run of another name**, as `launched` does. No report shows a session misnaming a return, and a return with an unknown or another name's ID is handled as now (`yagni`).
- **Not taken: accepting the defect,** the freeze's default answer. A rebuild on a late report puts two subagents on one branch, which nothing in the loop notices; whether to build the fix now is the human's, through the existing question.

## Outcome

- `orchestrator.loop` Contract: `clm build returned` takes an optional `--task <task-id>`.
- `orchestrator.loop` Design: the paragraph on `claim-gone-ends-a-build` says why a late return is ignored by its task, and what was not taken.
- `orchestrator.loop/returned-ignores-an-earlier-task` (new): a return whose `--task` is an earlier run's of its name changes nothing but the lease, finishes nothing, counts no stop and prints the actions to take now with none for it; a return with no `--task` or another ID is handled as before.
- `backlog/the-fix-for-a-landed-build-s-task-reported-for-its-successor-is-outside-the-freeze` (`question`) now blocks the new requirement too, since it relies on the IDs `launched-refuses-an-earlier-task` keeps.
- `backlog/the-build-skill-doesn-t-say-what-a-refused-clm-build-launched-means` (`deferred`, `plugin`) now also says that the skill's reporting sentence passes `--task`, once both requirements are built; it is prompt text, which no requirement states (`prompt-text-is-source`).

## Constraints

- No action line, run name, prompt file or loop file field changes: the requirement reads the earlier task IDs `launched-refuses-an-earlier-task` adds to the loop file.
- The flag is optional, so the Contract change is additive and the capability test `capability/build-from-spec`, which reports returns without it, sees nothing different.
- `return-rules`' commit-time rule for a `landed` line stays, for returns that name no task.

## Edge cases

- **A return naming the run's own task, or an ID the loop doesn't keep** is handled by `return-rules`, as now.
- **A return for a planned run** (the new run printed, not yet launched) whose task is an earlier run's is ignored too, and the step's `launch` is printed again unchanged.
- **A session running the skill's old text** passes no `--task`, and the defect stays for it until the skill's sentence is built.

## Open questions

None.
