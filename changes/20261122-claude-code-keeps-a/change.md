---
status: applied
resolves: [claude-code-keeps-a-finished-subagent-s-worktree-lock-for-the]
---

## Intent

A landed build's worktree goes once its subagent has handed back, so the build loop can build the component again in the same session. The build loop tells the finish the run has returned: `clm build returned` runs `clm run --finish <component> --returned` on a build's return, also for a run it finished earlier on a gone claim, and that finish removes the worktree whatever its lock says. A finish without `--returned` (the loop's on a gone claim, a skill's, the human's) still keeps a worktree whose lock a running process holds, so a build still waiting on its land keeps its worktree. Resolves `backlog/claude-code-keeps-a-finished-subagent-s-worktree-lock-for-the`.

## Problem

`20261122-clm-build-step-s` made `clm run --finish` keep a clean landed worktree whose lock a running process holds (`vcs/held-lock`), on the premise that Claude Code drops a subagent's lock once the subagent hands back. It doesn't. Claude Code writes every subagent's lock as `claude agent agent-<id> (pid <n> start <t>)` with `<n>` the orchestrating session's own process, and keeps it until the session ends: on 2026-11-22 `git worktree list --porcelain` showed every worktree Claude Code had made for a subagent of the `/clm:build` session locked with the same process, finished and running subagents alike, the locks of subagents that handed back hours before included. Within a session the held lock is therefore always true, and tells a running run from a finished one not at all.

So no finish removed a landed build's worktree. The loop usually finishes a build first on its gone claim (`claim-gone-ends-a-build`), when its `clm land` pushes `main`, and then marks it `returned`; the subagent's own return, arriving later, found the run `returned` and finished nothing a second time. The worktree stayed on `impl/<component>`, and the next `clm run <component>` the loop launched was refused by its leftovers step (`clm run: <worktree> has the landed impl/verify checked out and a running process holds its lock ... run git worktree unlock <path> first`). The session unlocked four worktrees by hand to go on.

The two unlocked worktrees the first amendment of `a-held-lock-keeps-a-worktree` relied on were a returned fold's and a returned draft's; `clm draft --finish` unlocks before it removes, which may explain them, and either way the lock can't be read as "still running".

No change or code fix since 2026-11-15 fixes this: `20261122-clm-build-step-s` made it, and the other changes of the period with an `orchestrator.run` delta or no delta touch other things.

## Approaches

- **Taken: the loop tells the finish the run has returned.** The hand-back is the one sign a subagent is done, and only the orchestrating session sees it; the loop records it through `clm build returned`, which already runs the finish. `--returned` carries that fact to `clm run --finish`, which then removes the worktree whatever its lock says, as `clm draft --finish` already does for a draft. A finish without it honours the lock as today, so nothing else that runs a finish before a return can remove a live subagent's worktree. `20261122-clm-build-step-s` rejected a flag on the claim-gone finish because "it says with a new flag what the lock already says"; the lock turns out to say nothing within a session, and the flag here is on the other call, so the safe default stays.
- **Not taken: the finish ignores a held lock unless told the run may still run (a flag on the claim-gone finish).** The default would remove a live subagent's worktree for any caller that doesn't know to pass the flag, which is the loss of four reports `20261122-clm-build-step-s` mended.
- **Not taken: each build unlocks its own worktree as its last step.** A rule in a model's hands (`build-loop-in-clm`), skipped by a subagent that ends without its closing line; and Claude Code may refuse a subagent's git call on its own worktree's lock.
- **Not taken: match a worktree to its task by the `agent-<id>` in its path and lock.** The agent ID is Claude Code's naming, not the task ID the loop records, and another agent harness would need its own matching (`agent-harness-adapters`).
- **Not taken: drop the held-lock rule from wrap-up and the leftovers step too.** Across sessions the lock still tells a live session's worktree from a gone one's, which is what those steps rely on; within a session they now leave the worktree to the return's finish, which loses nothing.
- **Not taken: keep the workaround** (`git worktree unlock` by hand). Every component the loop builds again in a session stops on it.

## Outcome

- `orchestrator.run/run-finish` (modified): `--returned` says the build's subagent has handed back; with it a held lock keeps no worktree, which is unlocked and removed as usual. `--returned` without `--finish` is a usage error. New scenario *a returned build's worktree removed*; *a running build's worktree kept* (no `--returned`) stays as it is.
- `orchestrator.run`'s Contract and Design say when the loop passes `--returned` and why the lock can't tell; the Design's leftovers paragraph says the return's finish removes a held leftover.
- `orchestrator.loop/return-rules` (modified): every `clm run --finish` that `clm build returned` runs for a build's return passes `--returned`; a return for a build run already listed as `returned` (the loop finished it on its gone claim) changes nothing in the loop file but the lease, prints no `stop` or `resume`, and, unless its line is `waiting: <command>`, runs `clm run --finish <component> --returned`, saying each `left` line. It `#### Needs` `orchestrator.run/run-finish`. Two new scenarios.
- `orchestrator.loop`'s Design ("A build that is over with no return") says the return's finish, not the lock, lets the worktree go.
- `orchestrator.checkpoint`'s Design no longer says Claude Code drops a subagent's lock on hand-back.
- The decision `a-held-lock-keeps-a-worktree` is amended again, and `build-places`' "Cleanup on land" says the finish on a return passes `--returned`.

## Constraints

- `claim-gone-ends-a-build` is unchanged: its finish passes no `--returned`, so a build still waiting on its land keeps its worktree, as now.
- `clm wrap-up` and `clm run`'s leftovers step are unchanged. Within a session they never remove a subagent's worktree (its lock is always held), which is now the return's finish's job; the leftovers step still refuses a component whose landed worktree a run that hasn't returned holds, naming it.
- `clm draft --finish` is unchanged: it already unlocks first. `release` and `loop-start`'s finish of an orphan are unchanged and pass no `--returned`: a released build's branch is moved away and isn't landed, and an orphan's session has gone, so its lock isn't held.
- None of the six `orchestrator.loop` requirements the plan item blocks is touched.

### Edge cases and workarounds

- A return that never reaches the loop leaves the worktree until the session has ended, and `clm run <component>` refuses meanwhile, naming the worktree and `git worktree unlock`; the loop passes that to a `say`. The question `a-worktree-the-finish-keeps-for-a-held-lock-is-told-to-no-one` covers telling the human.
- A `--returned` finish removes every clean worktree under `.claude/worktrees/` that has the landed `impl/<component>` checked out. Git checks a branch out in one worktree at a time, so while the returned build's worktree holds `impl/<component>` no other run's can; only a build that left its branch before handing back could have another run's worktree removed, which no build prompt does.
- A return with `--cause running` for a run the loop already finished prints no `stop`, so a background task the subagent left (its land, which has ended by then) is not stopped; its worktree is removed under it.
- A return for a `returned` build run re-runs the finish each time it is repeated; a second finish finds no place and no worktree and prints only `landed impl/<component>`.

## Open questions

- `backlog/a-worktree-the-finish-keeps-for-a-held-lock-is-told-to-no-one` (not claimed by this draft) asks whether Claude Code drops a stopped subagent's lock: it doesn't drop any subagent's lock before its session ends. Under this change a worktree the claim-gone finish keeps goes with the return's finish, so its first question (telling the human about a `kept` line) remains only for a return that never arrives. Triage may want to narrow or answer it.
