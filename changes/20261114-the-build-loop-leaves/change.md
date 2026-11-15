---
status: applied
resolves:
  - the-build-loop-leaves-a-landed-run-s-worktree-for-the-human-to
---

## Intent

A landed build's worktree is removed without the human doing anything, also when something was still writing into it while git deleted it. `vcs`'s `remove_worktree`, the one removal `clm run --finish`, `clm wrap-up` and so the build loop share, finishes what `git worktree remove` left: when git has unregistered the worktree but its directory remains, it removes the rest itself and tries again for up to 5 seconds. The line telling the human to delete the directory by hand stays only for what the session's user can't remove at all.

## Problem

`clm build returned <name> --result "landed <sha>"` finishes the run with `clm run --finish`, which removes the build's worktree through `remove_worktree`. The session reports the return as soon as the subagent's final report arrives, and the agent harness ends the subagent only afterwards, so a process of the run can still add a file while git deletes the directory. `git worktree remove` deletes the directory first and unregisters the worktree whether or not that worked: it failed with `Directory not empty`, and the directory was then no worktree, so no later `git worktree remove` and no `clm wrap-up`, which reads git's list, would find it again. The loop passed `--finish`'s `left <path>: ...; delete the directory by hand, then run git worktree prune` line to a `say`, and the human had to do it. Cleaning up a build's worktree isn't theirs (`build-loop-in-clm`).

Is the design wrong? The loop's part is right: it finishes a run through `orchestrator.run`'s command and holds no worktree rule of its own. The fault is that the shared removal gives up at the first failure of a command that has no second try, so the fix belongs in `vcs/remove-worktree`, not in the loop.

## Approaches

- **Taken: `remove_worktree` removes what git left, and tries again briefly.** When the directory still exists after `git worktree remove`, and git listed a worktree there before the call and lists none after it, the function removes the rest with `std::fs::remove_dir_all` after its permission walk, trying again while the directory exists for at most 5 seconds. Git has by then judged the worktree removable and deleted what it tracked, so nothing is discarded that git wasn't already discarding. Both callers gain it with no change to their text, and so does `clm run`'s removal of an earlier build's worktree.
- **Not taken: the loop waits for the agent harness to end the subagent before it finishes the run.** The loop learns of a return only from the session's report, and not every agent harness says when a subagent has ended (`agent-harness-adapters`); a wait would be a guess, and would hold the run's place meanwhile.
- **Not taken: the loop records the paths that were left and tries again at each step.** State in the loop file and a second removal, for a case the retry covers (`yagni`).
- **Not taken: `clm wrap-up` sweeps `.claude/worktrees/` for directories git doesn't list.** It could remove a directory the agent harness is still making, or one the human put there.
- **Not taken: `git worktree remove --force`.** It removes a worktree holding uncommitted work, and fails the same way on a directory being written into.

## Outcome

- `vcs`: the requirement `remove-worktree` states the second removal, its two guards (git listed the worktree before and no longer does), the 5 seconds and what it returns; the Design says why.
- `orchestrator.run/run-finish` and `orchestrator.checkpoint/checkpoint` stay true as written: they still print `left <path>: ...` when `remove_worktree` returns an error, which now happens only for what can't be removed. Their tests of that line (a parent directory nobody can write to; a `git` that refuses) still hold, the first taking 5 seconds longer.
- `orchestrator.loop` doesn't change: it still passes a `left` line to a `say`, for the cases below.

## Constraints

- `clm run --finish` and `clm wrap-up` take up to 5 seconds longer on a worktree whose directory nothing can remove.
- 5 seconds is a constant, not a setting.

## Edge cases

- A process that keeps writing into the worktree for more than 5 seconds after its build's return was reported: the `left` line is still printed and the directory is the human's to delete. None is known; a build's prompt has it stop its background tasks before its final message.
- A file in the worktree owned by another user, or a parent directory the user can't write to: nothing the session runs can remove it, so the `left` line stays.
- A worktree git refuses (an initialised submodule, a file that isn't ignored): git still lists it, so it is left untouched, as today.
- A path that was never a worktree: nothing is removed, and git's error is returned.

## Open questions

None.
