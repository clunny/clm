---
title: "`clm run` resuming a branch removes its worktree even when a running process holds the worktree's lock"
kind: idea
components: [orchestrator.run]
---

`orchestrator.run/run` keeps a *landed* leftover worktree whose lock a running process holds (`vcs/held-lock`, `a-held-lock-keeps-a-worktree`), but on *resume* (a local `impl/<component>` that isn't landed and has held `origin/impl/<component>`'s tip) it still removes a clean worktree that has the branch checked out, unlocking it first, whatever its lock says. The `/code-review` of the build that added the held-lock rule raised this: a build subagent still working, with nothing uncommitted yet, on an unlanded `impl/b` whose claim has lapsed would lose its worktree under it if `clm run b` ran again.

**The lock can't tell the two cases apart.** Claude Code locks a subagent's worktree in the name of the session's own process and holds the lock until the session ends (`20261122-claude-code-keeps-a`). So the lock of a build the loop launched earlier in the same session is held by a running process whether its subagent is still working or has handed back. A check of the lock on resume, as first proposed here, would refuse every resume inside one session, which is how the loop resumes its own returned builds.

**What would tell them apart is a signal from the loop**, as `--returned` is for `clm run --finish`: the loop knows when a build's subagent has handed back, and only then says so. On resume, a worktree whose lock a running process holds would be kept, and the resume refused with a reason, unless the caller says the earlier build's subagent has handed back; with that word it is unlocked and removed as now. A resume run without it (the human's, a skill's, a later session's) could then never remove a live subagent's worktree.

The Design argues `clm run` is only called for components `clm next` lists, which have no live claim, and for the loop's own returned builds, so the case needs a lapsed claim and a still-running session. Whether to guard the resume path, and the flag's name, is a spec decision: the requirement's text states the removal on resume without the lock condition, so changing the code needs a delta to `orchestrator.run/run` first, and the loop's side (`orchestrator.loop`) passes the flag only for a build it has seen hand back (`code-fix-needs-a-written-rule`).
