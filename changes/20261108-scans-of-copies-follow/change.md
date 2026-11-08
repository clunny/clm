---
status: proposed
resolves: [scans-of-copies-follow-git-dir-to-the-clone]
---

## Intent

Make the scans of plain copies and the git `clm rebase` runs in its temporary worktree follow no `GIT_DIR` or `GIT_WORK_TREE` the environment sets.
