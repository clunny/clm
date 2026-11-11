---
title: "Backlog items filed on branches that haven't landed go stale or duplicate unseen"
kind: idea
components: [orchestrator, orchestrator.run, sync]
---

## Intent

In the first CLM project on Azure DevOps, two builds stopped on the same outside problem and each filed the same `kind: bug` item on its own branch. A third build fixed the problem on `main` meanwhile. When the stopped builds resumed, `clm rebase` carried the item forward: the first to land put a stale P1 bug on `main`, which needed a cleanup branch of its own, and the second hit an add/add conflict on the same file. Items on an unlanded branch are invisible to other builds and to the orchestrating session, so nobody can see a duplicate or retire an item that has gone stale.

Reported from that project's session on 2026-11-10. Related: `a-build-stopped-twice-by-the-same-outside-cause-is-rebuilt`.

## Open questions

- Should a build blocked by a problem outside its component report it to the orchestrator (its closing message) instead of committing an item, leaving the orchestrator to file one item?
- Should `clm rebase` flag an item a branch adds when `main` already has one with the same slug, or when it names a test that now passes?
- Is the orchestrator's queue the place to hold such reports until one lands?
