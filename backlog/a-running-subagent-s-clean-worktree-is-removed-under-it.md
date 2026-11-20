---
title: "A running subagent's clean worktree is removed under it"
kind: bug
components: [orchestrator.run]
---

Step: a loop draft subagent (isolated worktree under `.claude/worktrees/agent-<id>/`) that found a code-only fix, claimed the component with `clm claim <component> --fix`, landed it with `clm land`, and then went on to delete its now-empty spec branch.

What happened: partway through that cleanup the subagent's own worktree directory disappeared, so it could run nothing more and left an empty `spec/<id>` and `autosync/spec/<id>` on origin for the session to delete by hand. Its tree was clean (everything committed) at the time. The likeliest cause is a sweep that removes clean worktrees under `.claude/worktrees/` once a component's branch has landed (`clm run --finish` documents such a sweep), which cannot tell a finished build's worktree from one whose subagent is still running.

Expected: a clean worktree is removed only when the run that owns it has returned (or the agent harness no longer lists it as running), never because its tree is clean and some branch landed.

Rule involved: `builds-in-subagents`, `build-loop-in-clm`.
