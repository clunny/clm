---
title: "Deployment is downstream of main and the project's own; CLM owns the path to main"
date: 2026-11-06
status: accepted
---

CLM is the project's continuous integration: `clm land` merges each branch with the current `main`, checks the merged tree and pushes `main` only when it passes, one land at a time (`land-locally`). So every commit on `main` is deployable. Whether and when it is deployed is the project's choice, made downstream of `main` by its own pipelines, release step or nothing at all. CLM generates no deploy, never removes one, and doesn't decide whether a project deploys continuously or behind a human's release.

Why:
- Deploying is outward-facing and often hard to undo, and how much risk a project takes there depends on the project, not the method. Some teams deploy every green `main`; CLM itself releases by hand (`host-ci-for-releases`).
- With humans no longer reviewing code (`ai-writes-code`), the human checkpoints are intent (`humans-review-intent`) and, where a team wants one, the release. CLM keeps `main` trustworthy and leaves the second to the team.
- A host's pipelines triggered by a push to `main` keep running after a land, as deploys and as a backstop. They report; they don't decide a land.

Consequences:
- `/clm:init` moves a repository's build and test steps into `scripts/clm-check.sh` and keeps its deploy, release and scheduled workflows and pipelines. It warns when a deploy triggered by `main` has no path filter, since spec-only lands (`specs/`, `changes/`, `backlog/`) reach `main` too.
- Tests that need a deployed environment, such as integration tests on a test cluster, gate promotion after `main` by default. A team that needs an integration-green `main` runs them from `[land] check`, paying with slower, serial lands. Slow tests take the same choice; what happens when one fails after the land is `backlog/slow-tests-after-land`. The guide's `ci-and-deployment` page says how.

Rejected: a landing mode in which the host's pull request completion, gated by its pipelines, lands the branch. It reverses `land-locally`, a `method` constraint, and makes the host the decision-maker again (`host-adapters`).
