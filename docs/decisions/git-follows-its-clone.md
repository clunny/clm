---
title: "clm-git drops the repository variables outside the clone they name"
date: 2026-11-08
status: accepted
---

`clm-git` passes the process's environment on to every command. With `GIT_DIR` and `GIT_WORK_TREE` set, as in a git hook or a clone whose git directory is outside its work tree, git run in any other directory works on the clone those variables name. Change `20261108-scratch-repositories-follow-git` took two sites off it (`clm rebase`'s rename copy stopped being a repository; the superpowers fetch runs through `git_isolated`), and more were found: `clm-coverage`'s scan lists the clone's files when it scans a plain copy, `clm-check`'s file listing does the same, and `clm rebase`'s and `clm land`'s temporary worktrees check out and scan the clone's work tree instead of their own.

Now the layer decides once: a git command whose `root` lies outside the work tree the environment names runs as `git_isolated` runs it, with the variables `git rev-parse --local-env-vars` prints removed, while a variable the caller passes in `Options::env` is kept (`vcs/git-follows-its-clone`). The named work tree is what `git rev-parse --show-toplevel` prints in the process's directory with its environment, found once per process and only when either variable is set, so a process without them pays nothing.

Every directory CLM runs git in outside the clone is one CLM made for itself (a copy, a temporary worktree, a scratch repository), so none of them is meant to follow the variables, and the rule covers sites not yet found and sites not yet written (`reuse-first`). `git_isolated` stays for a command run inside the clone that makes a repository elsewhere (`clm-host`'s `git init --bare <path>`).

Rejected:
- **Fix each site.** The scans in `clm-coverage` and `clm-check` would each need to ask whether `root` is the work tree git names, and `clm-sync` and `clm-land` would each route their worktree's git through `git_isolated`; the same question asked in four crates, with the next site found the same way, after it misfires.
- **`clm` removes the variables at start-up.** It would break the case the variables exist for: a clone whose work tree has no `.git`, which git finds only through them.
