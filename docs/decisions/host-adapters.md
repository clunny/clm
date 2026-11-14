---
title: "A git host is an optional adapter behind clm; git alone is the platform"
date: 2026-10-26
status: accepted
---

CLM depends on git and an agent harness (`any-agent-harness`), not on a git host. A git host (GitHub, Azure DevOps, or a bare git server) is where the remote lives and, when it has one, where a page per branch shows the diff and records the merge. It runs no checks and makes no decisions: checks and landing are `clm land` (`land-locally`), and the records are the ledger (`per-writer-ledgers`).

Everything host-specific is one adapter, the `host` node, picked by the `host` setting, whose values are `github`, `azure` (`azure-through-az-cli`), `git` and `none`: open a page for a branch, set its body, delete a remote branch, file an issue. `git` and `none` answer every page and issue operation with nothing to do, so a repository on a plain server or with no server at all runs the same flow. Nothing in CLM waits on a page. A new host is a new adapter and nothing else: what `clm init` needs to know of a host, which host a remote's URL names and the steps a person takes to set a repository on it up, is the adapter's too (`host/host-for-remote`, `host/host-setup-steps`), so `init` holds no host's names. Only the list of accepted `host` values in `config` and `clm init --host` grows.

Two points go beyond the adapter the `host` setting picks:
- With `host = "none"` the clone keeps a remote of its own, a bare repository in its common git directory added as `origin`, so every command, claim and land runs through `origin` as on any other host, and `clm land` never lands on the local `main` alone (`none-keeps-its-own-remote`).
- `clm defect` files its reports through the `github` adapter whatever the `host` setting, since they go to CLM's repository on GitHub, not the project's host (`defects-file-on-github`).

`git-is-truth` has no carve-out for a host: no check, gate or approval reads anything but git. `web` keeps its GitHub App because that is how it signs people in and reads a repository it doesn't have on disk, and it reads the same ledgers everyone else does.

Rejected: an abstraction over hosted CI (the point is not to need one) and keeping labels as a mirror of the ledger on GitHub (two sources of the same fact, one of which someone will read).
