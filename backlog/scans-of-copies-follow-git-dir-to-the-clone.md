---
title: "Scans of copies and temporary worktrees follow GIT_DIR to the clone"
kind: bug
components: [check.coverage, sync]
---

`clm-git`'s `git` passes on `GIT_DIR` and `GIT_WORK_TREE`, so git run in a directory other than the clone those variables name works on the clone instead. Change `20261108-scratch-repositories-follow-git` takes the repositories `clm-sync` and `clm-run` made of their own off it; two places stay:

- `clm-coverage`'s `scan(root)` runs `git ls-files` in `root` whatever `root` is. On the plain copies `rename/replay` and `sync/regenerate-renames` make (no repository, so the scan should walk them), it lists the clone's work tree instead, files the copy lacks dropped, so a marker only the copy holds is missed and `rename`'s version rewrites can differ.
- `clm rebase`'s step 8 runs git in its temporary worktree (`git checkout --detach <HEAD>` when step 7 commits an item, and the check's scan there), which with the variables set acts on the clone's own work tree and `HEAD`. `clm land`'s temporary worktrees may share this.

Found drafting `20261108-scratch-repositories-follow-git`. One alternative to fixing each site: `clm-git`'s `git` itself drops the variables when `root` lies outside the work tree they name, which would also cover sites not yet found; that changes `vcs`'s Contract for every caller.
