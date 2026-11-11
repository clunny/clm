---
title: "Azure pull requests can fail to open, and the build's review record is lost with its worktree"
kind: bug
components: [host, orchestrator.run]
---

On a project with `host = "azure"`, three builds landed with no new pull request: one body exceeded Azure DevOps's 4000-character description limit and `az repos pr create` refused it; two got `az repos pr: no pullRequestId or repository.webUrl in the answer` from `clm pr open`. `clm land` went ahead each time, as designed (nothing waits on a pull request, `land-locally`), so the review record (mutants, reviewer findings, gaps left) lived only in the build worktree's `.clm/pr-body.md`, which `clm run --finish` deletes. The orchestrating session had to copy the files out by hand.

Expected: the Azure adapter fits or refuses a body over the host's limit up front (the build prompt saying so), and handles or explains a reply it can't parse; a build whose pull request didn't open doesn't lose its body when the worktree goes.
