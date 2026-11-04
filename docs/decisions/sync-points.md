---
title: "Sessions sync at fixed points, and a changed spec basis must be handled before work goes on"
date: 2026-10-22
status: superseded
superseded_by: versioned-markers
---

A session working while other PRs land on `main` takes them in at fixed points in its flow, never on a timer: a task session after each step of its step plan and before its PR; `propose` before writing deltas, around `clm apply` and on any conflict. Each point runs `clm sync`.

On a task branch, `clm sync` also tests the task's **basis**: the bundle hash its work is built on, recorded as `context` in the task's own status file, against the node's current bundle hash. `gate/task-done-fresh` tests the same. While they differ, `clm sync` fails with `basis changed` on every run, so the session can't carry on past it. `clm run` records the basis when it claims the task, and `clm plan`, run on the task's branch, updates it. It lives in the status file, not only in `plan.md`, because a parallel `plan` item can re-plan `main` and record the new context while the work was built on the old one.

How to respond is the session's judgment: re-plan with `clm plan`, then adapt the work to the new spec or throw it away and rebuild, whichever fits. Either way the result is judged the same, by `clm sync`, the checks and the gate, and the PR records which way it went under `## Sync`. When a task's PR is already open and its session has ended, `implement` re-runs `clm run`, which resumes in the item's worktree, rather than doing the work itself.

Rejected: syncing on a timer or from a hook after each commit, which can rebase mid-edit; and leaving it to `gate/task-done-fresh` alone, which catches a stale basis only after the work is done.
