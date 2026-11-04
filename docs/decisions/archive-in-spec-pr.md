---
title: "A change with nothing to build is archived in its own spec PR"
date: 2026-10-22
status: accepted
---

A change that needs no code, such as a design-only change with no requirements, would otherwise sit in the queue as a `plan` and an empty task, each costing a session and a PR. So its spec PR applies it, sets it `done` and runs `clm archive` before opening, and the change arrives on `main` under `changes/archive/<id>/`.

The gate treats such a *new archived change* as the change the PR adds: its files get `change-tiers`, its declared `gate` is checked, and its `## Significant` counts for `gate/significant`. Until now the gate read only `change.md` files outside the archive, so the citations of a change archived this way didn't count. What tells a new archived change from a finished one is how it got there: a code PR's archive move (`archive-move`) changes only `status:` lines of files already on `main`, and its `## Significant` was reviewed when the change was proposed, so it never counts again.

Alternatives considered:
- Letting `check/archive-done` accept a `done` change outside the archive. That leaves finished changes mixed with live ones, which `clm next` and `clm status` would have to skip.
- Keeping the change `applied` and running its `plan` and `finish` as usual. That works but spends two sessions and PRs on nothing.
