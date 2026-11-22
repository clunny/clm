---
title: "A worktree `clm run --finish` keeps for a held lock is told to no one"
kind: question
components: [orchestrator.loop, orchestrator.run]
---

Since `orchestrator.run/run-finish` keeps a landed build's worktree whose lock a running process holds, printing `kept <path>: a running process holds its lock` (`a-held-lock-keeps-a-worktree`), the loop's handling of the finish's output (`crates/clm-loop/src/returned.rs`) turns only `left` lines into `say` actions, so a `kept` line reaches no one.

The lock's reason names the orchestrating session's process, which runs as long as the session does, so the worktree goes only once Claude Code drops the lock, which it was seen to do when a subagent hands back. Whether it also drops it for a subagent that is stopped (`TaskStop`), crashes or ends without a hand-back is not known. If it doesn't, the worktree stays locked for the rest of the session: every later `clm wrap-up` and finish keeps it, and the next `clm run <component>` for that component exits 1 through the leftovers step's held-lock refusal, with the human never told why.

Found by the code review of `impl/orchestrator.run` (wave 3). Questions for a spec change: should the loop `say` a `kept ...: a running process holds its lock` line, or record the kept path so a later step (the drained loop's `clm wrap-up`) re-runs the finish; and does Claude Code drop a stopped subagent's lock.
