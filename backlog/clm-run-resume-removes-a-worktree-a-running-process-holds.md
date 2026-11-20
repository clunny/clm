---
title: "`clm run` resuming a branch removes its worktree even when a running process holds the worktree's lock"
kind: idea
components: [orchestrator.run]
---

`orchestrator.run/run` keeps a *landed* leftover worktree whose lock a running process holds (`vcs/held-lock`, `a-held-lock-keeps-a-worktree`), but on *resume* (a local `impl/<component>` that isn't landed and has held `origin/impl/<component>`'s tip) it still removes a clean worktree that has the branch checked out, unlocking it first, whatever its lock says. The `/code-review` of the build that added the held-lock rule raised this: a build subagent still working, uncommitted nothing yet, on an unlanded `impl/b` whose claim has lapsed would lose its worktree under it if `clm run b` ran again.

The Design argues `clm run` is only called for components `clm next` lists, which have no live claim, and for the loop's own returned builds, so the case needs a lapsed claim and a still-running session. Whether the held-lock test should also guard the resume path is a spec decision: the requirement's text states the removal on resume without the lock condition, so changing the code needs a delta to `orchestrator.run/run` first (`code-fix-needs-a-written-rule`).
