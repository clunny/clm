---
title: "Claude Code keeps a finished subagent's worktree lock for the whole session, so run-finish keeps every landed build's worktree"
kind: bug
components: [orchestrator.run]
---

What happened: after `orchestrator.run/run-finish` gained the held-lock rule (a clean landed worktree whose lock a running process holds is kept), `/clm:build` reported a build's landing with `clm build returned`, and the loop's next launch of that component was refused: `clm build: skipped build:verify: clm run: <worktree> has the landed impl/verify checked out and a running process holds its lock, so it may still be in use; ... run git worktree unlock <path> first`. The subagent had already handed back its final report. `git worktree list --porcelain` showed every worktree Claude Code made for a subagent in the session locked with the reason `claude agent agent-<id> (pid <the session's own process> start <n>)`, finished and running subagents alike: Claude Code names the orchestrating session's process in the lock, not the subagent's, and does not unlock when the subagent hands back. So the held lock is true for the whole session and tells a running run from a finished one not at all.

Expected: a landed build whose subagent has handed back is finished and its worktree removed (or at least not counted as in use), so the loop can claim the component again; and a run still waiting on its land keeps its worktree.

Rules involved: `orchestrator.run/run-finish` and its Needs `vcs/held-lock`, the decisions `a-held-lock-keeps-a-worktree` and `build-places` (both assume Claude Code drops a subagent's lock when it hands back, which a draft inferred from an old draft's unlocked worktree), and `orchestrator.loop/claim-gone-ends-a-build`. Workaround used in the session: `git worktree unlock <path>` by hand, as the refusal says, for each finished run's worktree, before the loop can launch that component again. The backlog question `a-worktree-the-finish-keeps-for-a-held-lock-is-told-to-no-one` is related.
