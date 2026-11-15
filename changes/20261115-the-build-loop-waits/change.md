---
status: applied
resolves: [the-build-loop-waits-without-end-on-a-run-that-has-landed-when]
---

## Intent

A build loop must not wait for ever on a run that is over. `clm build` finishes a running build itself once the build's claim has left `origin`, whether or not the subagent's report ever reaches the session, and it finishes a run a previous loop left when that run can't be relaunched, instead of keeping it as work in progress. Both rules are code in the loop; the session is told nothing new.

## Problem

On 2026-11-15 one `/clm:build` session met three faults of the same kind: the loop held a run as busy that had nothing left to run.

- **A landed build whose report was lost.** A build's subagent landed its branch, and the agent harness never delivered its final report. The loop finishes a run only on `clm build returned`, so it listed the run as running and printed `wait` for about 50 minutes, until the session read `origin/main` itself and reported `landed <sha>`. No requirement says what the loop does with a run whose return never arrives.
- **A run of an earlier loop that can't be relaunched.** `clm build start` marks every run the previous loop left as an orphan to relaunch (`loop-start`). The relaunch of a build orphan is `clm run <component>`, which refused because the component had nothing pending. The step printed `skipped` and kept the orphan, and any orphan counts as busy, so every step relaunched and skipped it again, `clm view queue` showed it as running for hours, and a drain could never end.
- **No report gets it out.** The session held no subagent for that orphan, so it had nothing to report. `clm build returned <name> --result none --cause tool` paused the run for a resume that is never sent to an orphan.

The item also expects a loop with nothing running and nothing ready to print `end`. That part stays as designed: the loop is continuous and ends only on a drain or a stop (`build-loop-in-clm`, `build-stop`). What was wrong is that the stuck orphan kept a requested drain from ever ending, which this change fixes.

## Approaches

**For a build that is over with no return**

- **Taken: the loop finishes a running build whose claim has left `origin`.** `clm run` pushes the claim before the `launch` is printed, and only landing or a deletion by hand removes it, so "no `origin/impl/<component>`, or one whose tip is on `origin/main`" means the build's work is over. The step finishes it as a run that returned with no closing line, which `clm run --finish` already judges as landed or not (`unclosed-build-reads-finish`). The test reads git alone and holds for as long as the run is listed, so no single step has to catch the land (`git-is-truth`).
- **Not taken: an action asking the session to check whether the run is over** (the item's second suggestion). It puts a rule of the loop back into a model's hands, which is what `build-loop-in-clm` removed.
- **Not taken: a time limit on a run.** It can't tell a long build from a lost report, and it needs a number nobody can choose well.
- **Not taken: reading `clm wrap-up`'s `removed impl/<component>` line.** It is printed once, by whichever session's wrap-up removes the branch, and not at all when the worktree is left dirty.
- **Not taken: the same rule for drafts.** A draft whose spec change has landed may still be fixing an item on a code branch, so a landed change doesn't mean the draft is over, and no draft's lost report has been seen.

**For an orphan that can't be relaunched**

- **Taken: an orphan whose relaunch is refused is finished and returned.** A refused relaunch (`clm run` or `clm draft --resume` exiting 1) means there is no subagent and no way to start one. The step finishes the run as one that returned with no closing line, which frees its place, tells the human the reason once unless the finish finds the run landed, and leaves the component or items to the plan like any other.
- **Not taken: dropping only an orphan whose component has converged.** The loop would have to read the reason from `clm run`'s message or ask the queue for the component's state, and every other refusal (a dirty worktree, a claim held elsewhere, a local branch nobody pushed) would still keep the loop busy for ever.
- **Not taken: no longer counting orphans as busy.** A drain would then end while a run that can be relaunched hasn't been.

## Outcome

- `orchestrator.loop/claim-gone-ends-a-build` (new): every `clm build` command that prints actions first finishes each running build whose claim has left `origin`, before the housekeeping removes the local branch that tells a landed build from a deleted one.
- `orchestrator.loop/build-wait`: `clm build wait` also wakes when a running build's claim has left `origin`, so the step that finishes it runs within seconds of the land.
- `orchestrator.loop/return-rules`: a `landed <sha>` report for a build whose claim is on `origin` and unmerged changes nothing. It is the late report of a run the loop already finished, arriving after the plan launched the component again under the same name.
- `orchestrator.loop/loop-start`: an orphan whose relaunch is refused is finished and recorded as returned, with one `say` naming the reason unless it landed.
- The Design says why, with the approaches not taken.
- No capability test sees a difference: `capability/build-from-spec` drains its queue with every return reported.

## Constraints

- The loop still depends on the session's report for every run that doesn't land: a build that stops, and every draft, fold and archive.
- The loop drives `clm run --finish` and `clm draft --finish` through their Contracts as before, and holds no rule of theirs.

## Edge cases

- **The report arrives after the loop finished the run.** A report for a run that is `returned` already changes nothing (`return-rules`' code path for a repeated report). When the plan has launched the component again meanwhile, the new rule of `return-rules` ignores the late `landed` line by the new run's unmerged claim.
- **The claim is gone and no local branch is left**, as when another session's `clm wrap-up` in the clone removed it first. The finish can't tell whether the build landed, so the run is returned without being counted as landed. It is then missing from the end summary's list of what landed, and the verify run after a land waits for the loop to be idle. Accepted: both are reports, and nothing is built or lost by it.
- **A claim deleted by hand while the subagent is alive.** The loop finishes the run and frees its place; if the subagent pushes again, the branch is a claim no run of the loop holds, as after any hand deletion. `clm build release` is the stated way to give a build up.
- **A rebuild whose relaunch is refused.** A rebuild is relaunched as an orphan, so a refusal (usually the stopped build's uncommitted changes) now ends the run with a `say` instead of skipping it at every step. Its stop counts go with it, so a later build of the component starts at round 1.
- **Tests of the loop** start runs with a fake `clm run` that pushes no claim. Under the new rule such a run has no claim on `origin` and is finished at the next step, so the fixtures of the existing scenarios that need a live run give it a claim branch. That is test plumbing of the component's own suite.

## Open questions

- **This change is outside the plan's freeze, and approving it is the exception.** The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`) says the build loop and triage work only on the plan, that a defect's default answer is "accept", and that an exception is the human's to make. The change is proposed, not accepted, because the fault stalls the loop that builds the plan's own phases (50 minutes lost in one session, and a drain that can't end), as slow lands did before the landing exception. Declining it accepts the behaviour with these two steps by hand: report `landed <sha>` yourself for a build whose branch has merged, and run `clm build release <component>` for a run of an earlier loop that every step skips.
- **A refused relaunch is said once, not kept in the human's queue.** After the `say`, a component that `clm run` keeps refusing is skipped at every step with a line on stderr, as a refused build that was never an orphan is today. Putting refusals in the queue is a wider change than this bug needs.
- **A run that neither lands nor returns** (a subagent that died without a report, or a draft whose report is lost) still waits for the session's report. None has been seen, and no fact in git says such a run is over.
