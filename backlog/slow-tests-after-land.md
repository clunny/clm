---
title: "Slow tests after the land: a red run on main reaches the backlog, its land is found, and the line stops for that component"
kind: idea
components: [host, orchestrator.triage, land, gate]
---

Tests too slow for every land, which a team's pull-request pipeline runs today, can run after the land instead: a hosted pipeline runs them on `main`, per commit or batched, and reports without deciding (`deploy-is-downstream`). What CLM lacks is the way back when one fails after people have pulled `main`:
- **Into the backlog.** A failed run on `main` becomes a `kind: bug` item on the culprit's component through `clm intake`, as a dependency alert does (`dependency-alerts-through-intake`), the host adapter listing failed runs.
- **Finding the land.** Every land is a merge commit on `main`, so `git bisect --first-parent` between the last green and the red commit walks lands; the ledger's `landed` records give each one's change and components.
- **Fix forward or revert, both as lands.** The fix claims the component with `--fix` and adds a fast test reproducing the failure where it can. A revert (`git revert -m 1 <land>`) lands through `clm land`; it reverts the land's `req:` markers too, so its requirements show pending again and the queue rebuilds them, with the bug item saying what went wrong. Nothing rewrites `main`, so clones that pulled need nothing.
- **Stop the line, narrowly.** Rather than blocking every land while `main` is red on the slow tier, `clm land` could refuse code lands on that one component until the item is resolved, the item taking top priority. That is a gate rule, so it needs a spec.

Raised by the human on 2026-11-06 while designing `20261106-azure-devops-host`, for a project whose slow tests run in the pull request's pipeline before the merge.
