---
title: "clm wrap-up and the runner fail to remove a build worktree holding an unwritable directory"
kind: bug
components: [orchestrator.checkpoint, orchestrator.runner]
---

`20261106-run-finish-fails-on` has `clm run --finish` make a landed build's clean worktree removable before `git worktree remove` (owner read, write and execute on every directory under it, no symbolic link followed) and report a worktree it still can't remove as `left <path>: ...` instead of failing. Two other commands remove build worktrees the same way and trip on the same thing, a directory the build left without its owner's write bit (for instance by unpacking an archive into an ignored path): `clm wrap-up`'s removal of landed `agent-*` worktrees under `.claude/worktrees/` (`orchestrator.checkpoint`), and the runner's removal of its `agent-runner-<component>` worktree before it adds a new one (`orchestrator.runner`). There, git unregisters the worktree and fails to delete the files, and the command fails. Neither component's rules say what it does with such a directory, so the fix needs a spec change: the same treatment as `orchestrator.run/run-finish`, reusing its code where it fits.
