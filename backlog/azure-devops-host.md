---
title: "Azure DevOps repos: a replica source, sign-in and a host adapter"
kind: idea
components: [replica, host, web.worker]
---

Support repos hosted on Azure DevOps. The pieces:
- a `replica` source over its REST API: service hooks for push and pull-request events, the changed files of a push, and one file's text at a commit;
- sign-in and access checks through Microsoft Entra ID instead of the GitHub App;
- the approval label and review flow's equivalents, such as PR tags or required reviewers;
- an `azure` adapter in `host` beside `github`, for `clm pr open`, `pr body` and `issue` (`host-adapters`).

Raised by the human on 2026-11-02 while designing `20261102-web-repo-replica`, which keeps the replica's core host-neutral so this fits it.
