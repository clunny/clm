---
title: "A git host is an optional adapter behind clm; git alone is the platform"
date: 2026-10-26
status: accepted
---

Supersedes `github-and-claude-code`. CLM depends on git and Claude Code. A git host (GitHub today, a bare git server later; the Azure DevOps adapter this once planned was cut in `20261029-refounding`, since it had no user) is where the remote lives and, when it has one, where a page per branch shows the diff and records the merge. It runs no checks and makes no decisions: checks and landing are `clm land` (`land-locally`), and the records are the ledger (`per-writer-ledgers`).

Everything host-specific is one adapter, the `host` node, picked by the `host` setting: open a page for a branch, set its body, delete a remote branch, file an issue. `git` and `none` answer every operation with nothing to do, so a repository on a plain server or with no remote at all runs the same flow. Nothing in CLM waits on a page. A new host is a new adapter and nothing else.

`git-is-truth` loses its "and GitHub" carve-out: no check, gate or approval reads anything but git. `web` keeps its GitHub App because that is how it signs people in and reads a repository it doesn't have on disk, and it reads the same ledgers everyone else does.

Rejected: an abstraction over hosted CI (the point is not to need one) and keeping labels as a mirror of the ledger on GitHub (two sources of the same fact, one of which someone will read).
