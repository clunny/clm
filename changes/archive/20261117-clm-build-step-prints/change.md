---
status: done
resolves: [clm-build-step-prints-a-launch-for-a-run-that-is-still-running]
---

## Intent

`clm build launched` refuses a task ID that belongs to another run, or to an earlier run of the same name: the subagent of a build that landed before it reported, whose component the loop then builds again under the same name, is no longer recorded as the new build's. Its refusal says the printed `launch` is a new run, to be started as a new subagent. The loop goes on printing that `launch`, which is right. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

The item reports that after a `clm build wait` woke, `clm build step` printed `launch build:<component>` for a build launched about 15 minutes earlier, whose subagent was still running and whose claim was live, and rewrote its prompt file. The session took the line for a duplicate and ran `clm build launched` again with the old task ID, after which `step` printed `wait`.

The loop file and `origin` show what happened, with `view.repo` the component. Its build was claimed at 14:11 (`c095f522`) and landed at 14:23 (`3af12c7f`, "Merge impl/view.repo into main"); the subagent's final report hadn't come. At the next step `claim-gone-ends-a-build` finished `build:view.repo`, as it states, since `origin/impl/view.repo` was gone. `view.repo/arguments-and-exit` was still `unbuilt`, so `clm next` offered `view.repo` again, and the plan launched a new build under the same name: `clm run view.repo` claimed it afresh at 14:28 (`f2926be6`) and wrote the new prompt to the same file. So the loop didn't print a launch for a run that hadn't returned: it had finished that run, and the "live claim" was the new build's. The Design of `claim-gone-ends-a-build` already expects a report to arrive "after the plan has launched the component again under the same name", and `return-rules` ignores that report's `landed <sha>`.

What is wrong is that nothing tells the session so, and a run is named only by its name. The session then recorded the old task ID for the new run: the loop file now lists `build:view.repo` as running with the old subagent's task, while nothing builds the new claim. Nothing in the loop ends such a run: its claim stays on `origin`, so `claim-gone-ends-a-build` never fires, and the old subagent's `landed` report is ignored as late. The new claim is held until someone notices. No requirement states that `launched` refuses such a task ID, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-10 fixes it: `20261115-the-build-loop-waits` added `claim-gone-ends-a-build` itself, `20261116-the-build-loop-shows` the pause and the printed state in the queue, `20261117-the-build-loop-drafts` the draft that waits for its build, and `d62a4f0a` the skill's `run` bullet; none reads the task ID `launched` is given.

## Approaches

- **Taken: `launched` refuses a task ID the loop already gave to another run or to an earlier run of the name.** `launched` is the one point where the session's misreading turns into a wrong record, and the loop file already holds each run's task ID, so the check needs only the earlier runs' IDs kept when a step replaces a finished or rebuilt run under its name. It is a check in `clm` with a test (`build-loop-in-clm`, `deterministic-enforcement`), and its stderr carries the instruction the skill lacks to the session at the moment it needs it.
- **Not taken: not launching the component again while the landed build's subagent may still run.** The loop can't know when the subagent ends, since its report may never come, which is why `claim-gone-ends-a-build` exists; holding the build back would bring back the wait it removed.
- **Not taken: printing `stop <name> <task-id>` for the landed build's task.** A stop is reported back with `clm build returned <name> --cause stopped`, by name, which would mark the new run stopped; and a subagent still running `clm land`'s cleanup would be cut short for no gain.
- **Not taken: naming each run uniquely (`build:view.repo#2`).** Every command, the panel's prompt files (`loop run <name>`) and the queue read the name; a new form of name changes them all for a case one refusal covers (`yagni`).
- **Not taken: a sentence in the `build` skill only.** Prose is what a model got wrong here; with no check, the next session that reads the line as a duplicate does the same. The refusal's stderr says what the sentence would.
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. The outcome is a claim held with nothing building it, found only by the human; whether to build the fix now is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/launched-refuses-an-earlier-task` (new): `clm build launched <name> <task-id>` exits 1, changing nothing, when the task ID is one the loop file records other than the current run's own: another run's, or one kept as an earlier run's of any name. The loop keeps a run's task ID with its name whenever it clears or replaces it (a rebuild, an orphan's relaunch, a new run planned under a finished run's name). The stderr names the run the task belonged to and says the printed action is a new run, to be started as a new subagent (`launch`) or a new command (`run`).
- `capability/build-from-spec`'s test reports one task ID for two runs in `drain_the_queue` and `resume_in_a_new_session`; the requirement's own build gives each run its own ID, keeping the citations (`capability-tests-follow-rules`).
- `backlog/the-fix-for-a-landed-build-s-task-reported-for-its-successor-is-outside-the-freeze` (`kind: question`) blocks the requirement until the human answers.
- `backlog/the-build-skill-doesn-t-say-what-a-refused-clm-build-launched-means` (`kind: deferred`, `plugin`): the `build` skill's `launch` and `run` bullets say that a printed action is always a new run and what a refused `launched` means, once the requirement is built (the architecture audit's finding, `method-in-the-plugin`). It is prompt text, which no requirement states (`prompt-text-is-source`).
- `backlog/a-late-report-of-a-landed-build-s-subagent-is-recorded-for-the-next-build-of-its` (`kind: bug`): the edge case below, filed so that it isn't lost (`fold-small-defer-large`, the audit's finding).

## Constraints

- No action line, run name, Contract form or prompt file changes; `launched` gains a refusal, exit 1, which the Contract already gives for work that isn't possible.
- The loop file gains one optional field per run, so a file an earlier `clm` wrote loads unchanged and `version` stays 1; no migration is needed (`upgrades-carry-migrations`). `loop-file`'s list isn't amended: the new requirement states the field, and `loop-file` is already behind and blocked by the plan item.
- A refused `launched` doesn't renew the lease, as no refusing `clm build` command does (`pause`, a live `start`, a run the loop doesn't list), though `step-actions` says every command renews it.
- `view`'s queue model is unchanged.

## Edge cases

- **The old subagent's report.** A landed build's late `landed <sha>` is already ignored (`return-rules`). A report of another kind from it, such as `none --cause context`, would still be recorded for the new run, since `returned` names a run only by its name; it is filed as `backlog/a-late-report-of-a-landed-build-s-subagent-is-recorded-for-the-next-build-of-its`, outside this change, which only stops the wrong record at `launched`.
- **A task ID reported twice for the current run** is not refused: `launched` repeated for the same run stays harmless.
- **A new `start` forgets returned runs**, and their task IDs with them. Their subagents died with the session the start replaces, so no session reports them.
- **A different task ID for a run already `running`** is still recorded over the old one, as now.

## Open questions

None.
