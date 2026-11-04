---
status: proposed
resolves: [backlog/dev-install-scoped-to-the-clone]
---

## Intent

A contributor's clone of CLM's repository, with its worktrees, runs `origin/main`'s `clm` and the checkout's plugin, while every other project on the machine runs the release. Today `scripts/dev-install.sh` reaches further than its clone.
