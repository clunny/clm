---
status: applied
resolves: [a-running-subagent-s-clean-worktree-is-removed-under-it]
---

## Intent

A worktree is removed only once the run working in it is over. `clm wrap-up` and `clm run`'s leftovers step keep a landed, clean worktree whose lock names a process that still runs, which is how Claude Code marks the worktree of each subagent its live session launched; a worktree locked by a session that has gone, or locked naming no process, is removed as before. Resolves `backlog/a-running-subagent-s-clean-worktree-is-removed-under-it`.

## Problem

On 2026-11-19 a loop draft fixed a bug by component: it claimed `impl/<component>` with `clm claim --fix`, landed it from its own worktree, and went on to delete its now-empty spec branch. Partway through, its worktree directory disappeared, so it could run nothing more and left `spec/<id>` and `autosync/spec/<id>` on `origin` for the session to delete by hand.

`orchestrator.checkpoint/checkpoint` removes each clean worktree in `.claude/worktrees/agent-*` on an `impl/*` branch that has landed, and every step of the build loop runs `clm wrap-up` (`orchestrator.loop`). The draft's worktree was exactly that from the moment its land pushed `main` until it switched off `impl/<component>`. `orchestrator.run/run`'s leftovers step removes the same kind of worktree when `clm run <component>` starts, so a build launched for that component in the same window would have done it too. Both read "landed" as "over", and neither a draft after its fix nor a build writing its report after its land is over.

## Approaches

- **Honour a lock held by a running process** (chosen; `a-held-lock-keeps-a-worktree`). `vcs`'s `worktrees` says whether each worktree's lock reason names a `pid <n>` that still runs (`vcs/held-lock`); `clm wrap-up` and `clm run`'s leftovers step remove no worktree whose lock is held. Claude Code writes `claude agent agent-<id> (pid <n> start <t>)` on each subagent's worktree, `<n>` its session's process, so a live session's worktrees are kept and a gone session's are removed. A live session's returned runs are removed by `clm run --finish` and `clm draft --finish`, which run on each return and unlock first, as today.
- **Have the loop's step skip worktree removal.** Rejected: another session's wrap-up, or the human's, removes the same worktree.
- **Track running runs' worktrees in the build places.** Rejected: a build's place stops counting when it lands, the moment in question, and a draft's place doesn't name the `impl/*` branch of its fix.
- **Have the draft switch back to `spec/<id>` before landing its fix.** Rejected as the fix: it closes one path and leaves a build after its land, and the next run that lands from its worktree, exposed.

## Outcome

- `vcs/held-lock` (added): `worktrees(root)` gives each worktree whether its lock is held.
- `orchestrator.checkpoint/checkpoint` (modified): a landed build's worktree whose lock is held stays, with its local branch; two scenarios prove the kept and the removed case.
- `orchestrator.run/run` (modified): a landed leftover whose lock is held is kept, and `clm run <component>` exits 1 naming its path and saying a running process holds its lock, without claiming.
- `orchestrator.checkpoint`'s and `orchestrator.run`'s Designs say why, and `a-held-lock-keeps-a-worktree` names the sentence of `builds-in-subagents` it narrows.

## Constraints

- The liveness test is the operating system's: on Unix, `kill(pid, 0)` succeeding or failing with `EPERM` means the process runs, `ESRCH` that it doesn't, through `libc` behind `cfg(unix)`, which other crates of the workspace already depend on. Elsewhere a lock naming a process ID counts as held. `clm-git` stays out of the wasm builds, so this adds nothing there.
- `Worktree` gains a field; no crate outside `clm-git` builds one by struct literal, so the addition breaks no user.
- A lock with no reason, or a reason naming no `pid <n>`, is judged as today, so the scenarios that lock a worktree with a plain `git worktree lock` (`landed work cleaned up`, `landed leftover worktree`, `stopped after a rebase`) keep their outcomes.

### Edge cases and workarounds

- A reused process ID keeps a worktree until that process ends. Accepted: it loses nothing, and the next wrap-up after that removes it.
- A live session's own wrap-up keeps its worktrees whose run `--finish` didn't remove; the next wrap-up after the session has gone removes them.
- `clm run`'s removal of an unlanded worktree when it resumes a branch is unchanged: the loop calls it for its own returned builds, whose worktrees its live session still locks, and for orphans, whose session has gone.

## Open questions

None.
