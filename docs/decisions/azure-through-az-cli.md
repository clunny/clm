---
title: "The Azure DevOps adapter runs the Azure CLI"
date: 2026-11-06
status: accepted
---

`host = "azure"` opens and updates pull requests through the Azure CLI with its `azure-devops` extension (`az repos pr list`, `create` and `update`, with `--detect true`), run through `clm-git`'s layer, as `github` runs `gh` (`host-adapters`, `host/git-through-layer`). `az` holds the sign-in, through `az login` or `AZURE_DEVOPS_EXT_PAT`, so CLM reads, stores and scopes no token. The adapter offers only pull requests: `issue` says there is nowhere to file, since `clm defect`'s reports belong on CLM's own GitHub repository, and forks, contribution listing and dependency alerts report that the host has none, until a capability asks for them (`yagni`).

Rejected:
- The Azure DevOps REST API with a personal access token: no CLI to install, but `clm-host` would gain an HTTP client and CLM would handle a secret, for the same two operations.
- No adapter, with Azure users on `host = "git"`: everything but opening pull requests already works that way, but reviewers would lose the page where they read a spec diff, the one thing a host adds.
- Filing `clm issue` as an Azure Boards work item: the process template decides the work item types, and the reports are about CLM, not the project.

The web app's Azure support (a replica source over the REST API, sign-in through Microsoft Entra ID, the review flow) is a separate idea, `backlog/azure-devops-web`.
