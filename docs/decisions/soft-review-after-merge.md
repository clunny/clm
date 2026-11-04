---
title: "SOFT spec PRs are reviewed after they merge"
date: 2026-10-22
status: superseded
superseded_by: per-writer-ledgers
---

A SOFT spec PR merges as soon as its checks pass. CI then labels the merged PR `spec-review` and comments asking for review. A human reviews it when they can and removes the label; if they disagree, the fix is a new change, like any other spec edit. `gh pr list --state merged --label spec-review` lists what is waiting.

Alternatives considered: a review window (a scheduled workflow merges a green SOFT PR after 24 hours unless a human adds `spec-hold`), which gives a look before merge but delays every change and adds a workflow to maintain; and treating SOFT as HARD for now, which keeps today's approval cost. Merging first matches the paper's worked example ("auto-approved, human notified asynchronously").
