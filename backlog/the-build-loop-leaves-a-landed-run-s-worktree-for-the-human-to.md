---
title: "The build loop leaves a landed run's worktree for the human to delete by hand"
kind: bug
components: [orchestrator.loop]
---

Step: `clm build returned <name> --result "landed <sha>"`, run by the /clm:build session as soon as the subagent's final report arrived.

What happened: the loop tried to remove the run's worktree, `git worktree remove` failed with "Directory not empty", and the loop printed a `say` action telling the human to delete the directory by hand and then run `git worktree prune`.

Expected: a landed run's worktree is removed without the human doing anything, for example by retrying the removal later or removing what is left of the directory itself. The agent harness reported the subagent as finished only after the return was reported, so the run may still have been writing into the worktree when the removal ran.

Rule involved: humans own intent only; cleaning up a build's worktree is not theirs (`build-loop-in-clm`).
