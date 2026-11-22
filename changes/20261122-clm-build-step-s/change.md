---
status: applied
resolves: [clm-build-step-s-wrap-up-removes-a-landed-build-s-worktree]
---

## Intent

A landed build's worktree stays until its subagent has handed back, so its report always reaches the session. `clm run --finish <component>` keeps a worktree whose lock a running process holds (`vcs/held-lock`), as `clm wrap-up` and `clm run`'s leftovers step already do, printing `kept <path>: a running process holds its lock`; it still frees the place and prints `landed impl/<component>`. The build loop runs that finish when a build's claim has left `origin`, which is the moment the build's own `clm land` pushes `main`, and that finish is what removed the worktrees. The loop itself changes no rule: only its Design's claim that finishing early changes nothing is corrected, with `orchestrator.checkpoint`'s Design and the decision `a-held-lock-keeps-a-worktree`. Resolves `backlog/clm-build-step-s-wrap-up-removes-a-landed-build-s-worktree`.

## Problem

During a `/clm:build` run on 2026-11-22 four builds lost their final reports. Each ran `clm land` in the background, as its prompt says, and ended its turn waiting for the land's completion notice. The land pushed `main` and deleted `origin/impl/<component>`; `clm build wait` woke on the claim having left `origin` (`build-wait`), and the next `clm build step` finished the build (`claim-gone-ends-a-build`) by running `clm run --finish <component>`, which removes each clean worktree with `impl/<component>` checked out, "unlocking it first" (`run-finish`). Claude Code then refused to resume the subagent, its worktree being gone, so its review findings, mutation checks, defects and pull request body never arrived.

The item names `clm wrap-up`, but the wrap-up the step runs after the finish keeps such a worktree: since `20261119-a-running-subagent-s` it removes no worktree whose lock is held by a running process (`orchestrator.checkpoint/checkpoint`, `vcs/held-lock`), and Claude Code locks each subagent's worktree naming its session's process. The finish is the one removal that ignores the lock: the decision `a-held-lock-keeps-a-worktree` left `--finish` unchanged on the premise that it "runs once the run has returned", which `claim-gone-ends-a-build` breaks. The loop's Design says "finishing early changes nothing a prompt return would have done, since the housekeeping removes a landed build's worktree at the same step anyway", which has been false since that change.

What the lock says, seen in this clone on 2026-11-22: the worktrees of `build:host` and `build:land`, running with their branches already landed on `main`, were locked as `claude agent agent-<id> (pid <session> start <t>)`, while the worktree of `fold:20261122-a-build-that-narrows`, which had returned, and of a draft of 2026-11-19 that had returned, had no lock. So Claude Code holds the lock while its subagent runs, between its turns included, and drops it once the subagent has handed back.

No change or code fix since 2026-11-15 fixes it: `20261119-a-running-subagent-s` gave wrap-up and `clm run`'s leftovers the lock rule and left `--finish` out; the loop's later changes (`20261119-the-build-loop-keeps`, `20261119-the-build-loop-launches`, `20261122-a-deferred-item-that` and the live `20261116`/`20261117` ones) touch the queue, drafts, task IDs and ordering; the code fixes since (`7c54279d`, `b94f4a2e`) touch wrap-up's deletes and verify's places.

## Approaches

- **Taken: `clm run --finish` honours a held lock** (`reuse-first`). It is the rule `clm wrap-up` and `clm run`'s leftovers step already follow, through the same `vcs` query, so every removal of a build's worktree reads the lock the same way. A finish on a return removes the worktree as before, since the lock is gone by then; a finish on a gone claim keeps it, and the next step's `clm wrap-up` removes it once the subagent has handed back and its lock is dropped. No new state, option or loop rule.
- **Not taken: the loop's claim-gone finish passes an option to keep the worktree.** It says with a new flag what the lock already says, and leaves a finish run by anything else before a return (a skill, the human) removing a live subagent's worktree.
- **Not taken: drop `claim-gone-ends-a-build`, finishing a build only on its return.** A report the agent harness never delivers would again hold the run as running for ever, the fault that requirement was made for, and the drain could not end.
- **Not taken: the step's `clm wrap-up` keeps the worktrees of the runs the loop lists as running.** Wrap-up doesn't remove them anyway while their lock is held, and it would need the loop file in `orchestrator.checkpoint`, whose decision `a-held-lock-keeps-a-worktree` rejected tracking runs' worktrees for the same reason.
- **Not taken: the build prompt runs `clm land` in the foreground.** A land queues behind others' lands for longer than a tool call's time limit, and the bug would stay for any run that lands and then reports.

## Outcome

- `orchestrator.run/run-finish` (modified): a clean worktree with `impl/<component>` checked out whose lock is held is kept, locked, with `kept <path>: a running process holds its lock`; the place is freed and `landed impl/<component>` printed as before. A new scenario, *a running build's worktree kept*, proves it; *landed build tidied*, with a plain lock naming no process, still proves the removal. It now needs `vcs/held-lock`, which is current.
- `orchestrator.run`'s Contract and Design say when the finish keeps a worktree and why.
- `orchestrator.checkpoint`'s Design says Claude Code drops a subagent's lock once it hands back, so a returned run's worktree is removed by its finish or the next wrap-up, and that `clm run --finish` keeps a held one too.
- `orchestrator.loop`'s Design no longer claims finishing early changes nothing; `claim-gone-ends-a-build` and every other loop requirement are unchanged, so none of the six requirements the plan's freeze holds is touched, and `orchestrator.loop` gets nothing to build.
- The decision `a-held-lock-keeps-a-worktree` is amended: `clm run --finish` honours the lock, and why.

## Constraints

- The loop reads only `freed`, `landed impl/<component>` and `left ` lines from `clm run --finish`; a new `kept` line changes nothing it does, as the `kept <path>: uncommitted changes` line doesn't today.
- `clm draft --finish` is unchanged: the loop runs it only on a draft's return, on an orphan whose session is gone, or on a draft no subagent started (`draft-of-gone-items-dropped`), none of which holds a live lock.

### Edge cases and workarounds

- A build of the same component the loop prepares while the earlier subagent still holds its landed worktree is refused by `clm run`'s leftovers step (exit 1, naming the held worktree), and the step skips it until a later step, once the subagent has handed back. That window is the subagent's last minutes, and the rule already exists for wrap-up's sake.
- A late report of a run the loop finished on its gone claim still runs as today; the worktree goes at the next step's wrap-up, or the report's own finish, whichever comes first after the lock is dropped.
- An agent harness that dropped a subagent's lock while the subagent still waited between turns would leave its worktree unprotected, as wrap-up already is. Claude Code doesn't, as seen above; the lock is the harness's word, and nothing else in git tells a running run from a finished one.

## Open questions

None.
