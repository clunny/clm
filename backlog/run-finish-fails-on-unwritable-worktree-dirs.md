---
title: "clm run --finish loses its output and leaves files when a landed build's worktree has unwritable directories"
kind: bug
components: [orchestrator.run]
issues: [205]
---

A build can leave a directory without its owner's write bit inside its worktree, in an ignored path, for instance by unpacking an archive. `git status` still reads the worktree as clean, so `clm run --finish` asks git to remove it. Git unregisters the worktree but can't delete those files. `finish` has already freed the place, but it drops the `freed` and `landed` lines it had gathered and exits 2. On the next run the worktree is no longer listed, so it prints `landed` and says nothing of the directory still on disk, and a person has to fix the permissions and delete it by hand.

Expected: on the first run, `clm run --finish` deletes a landed build's worktree under `.claude/worktrees/` whole, including directories the build left without write permission, and prints its `freed`/`landed` lines. Where something still can't be removed, it should say which path is left and how to clear it, not fail silently. `run-finish` states neither behaviour today, so this needs a spec change first.
