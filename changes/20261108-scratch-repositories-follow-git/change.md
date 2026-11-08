---
status: proposed
resolves: [scratch-repositories-follow-git-dir-to-the-clone]
---

## Intent

Make the two repositories CLM makes of its own in a temporary directory, the copy `clm rebase` replays a rename in (`sync/regenerate-renames`) and the superpowers fetch (`orchestrator.run/superpowers-path`), follow no `GIT_DIR` or `GIT_WORK_TREE` the environment sets, by running their git through `clm-git`'s `git_isolated` (`vcs/git-isolated`). This resolves `scratch-repositories-follow-git-dir-to-the-clone`.
