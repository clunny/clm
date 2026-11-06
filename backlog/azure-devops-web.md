---
title: "Web app on Azure DevOps: a replica source, Entra ID sign-in and the review flow"
kind: idea
components: [replica, web, web.worker]
---

Let the web app follow and review repositories hosted on Azure DevOps, as it does GitHub's. The pieces:
- a `replica` source over Azure DevOps' REST API: service hooks for push and pull-request events, the changed files of a push, and one file's text at a commit; the replica's core is host-neutral for this (`web-repo-replica`);
- sign-in and access checks through Microsoft Entra ID beside the GitHub App, with the invite list (`invite-only-web`) naming accounts of either kind;
- the review flow's equivalents on Azure, such as a pull request's tags or votes.

Split out on 2026-11-06 from `azure-devops-host`, whose change `20261106-azure-devops-host` gave the CLI and plugin an `azure` host adapter (`azure-through-az-cli`).
