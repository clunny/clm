---
title: "One AI-owned plan per change, one task per node"
date: 2026-10-21
status: superseded
superseded_by: node-reconciliation
---

After a change's spec PR merges, Claude writes `changes/<id>/plan.md`. It holds one task per touched node. Each task lists its node, the requirements it must cite, the tasks it waits for (from `depends_on` within the change), and the hash of its `clm context` bundle when planned.

`clm check` validates the plan: every ADDED or MODIFIED requirement belongs to exactly one task, and a task whose recorded bundle hash no longer matches is stale and must be re-planned (`hash-based-staleness`). Node `paths` are already disjoint, so tasks that don't wait on each other can run in parallel.

A plan describes how to build, not what to build, so humans don't review it (`humans-review-intent`). It merges in its own code-class PR, which requires a gate rule for `plan.md`, and from there every machine can see it. Each task merges as its own code PR based on `main` (`no-stacked-prs`). A final PR marks the change `done` and archives it, which is when coverage is enforced.

Task per node, rather than per requirement, was chosen because it maps onto `paths` and keeps the number of PRs small.
