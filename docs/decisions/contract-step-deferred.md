---
title: "A contract step left proposed is tracked by a deferred backlog item"
date: 2026-11-01
status: accepted
---

`expand-contract` leaves the contract change `proposed` and unapplied until the requirements that move dependants off the old contract are current, and `contract-removal-needs` keeps that second change for the removals no `#### Needs` can order: a whole requirement under `REMOVED Requirements`, and, since `requirement-needs` forbids a need from a required node to an optional one (`optional-nodes`), a removal on a required node that waits on an optional consumer. `20261101-needs-pending-goes` is one: `orchestrator` drops `needs_pending` once `web/implementation-view` reads `waits_on`. Nothing CLM reads recorded that wait. A proposed change on `main` that names no backlog item gets no `clm triage` line, so the step survived only in the memory of some session.

Now the branch that proposes such a contract change also adds a `kind: deferred` backlog item for it. Its `nodes` are the contract change's nodes, and its body names the contract change and each requirement it waits on, with the version at which that requirement must be current, so whoever judges it compares against `clm pending` and `clm hashes`. The contract change's `resolves` doesn't name the item: an item a change on `main` names is shown as that change's `main` line, which says the change is applied and waits to be built. Left untaken, it is a `deferred` line in `clm triage`, which the `triage` skill shows to the human (`triage-owns-backlog-work`). When the human judges it due, the draft that takes it applies the contract change, rebasing its stale deltas first, and its own change resolves the item. The `propose` skill and the `clm draft` prompt both say so in their builder-check steps.

This reuses what exists: the `deferred` kind, its `triage` line and the draft's handling of a claimed `deferred` item. The contract change stays a change, because its `clm:remove-by` markers must name a live change (`check/remove-by-markers`).

Refines `expand-contract` and `contract-removal-needs`.

Rejected:
- **Let a required node's requirement need an optional one's.** `optional-nodes` forbids it so that CLM works without its optional parts. A required node then waits on a build the repository may never make. Relaxing it for removals alone would need a second kind of need.
- **A machine-read condition** (`waits_on: [<node>/<slug>@<version>]` on the item or the change, with `clm triage` printing it once due). `requirement-needs` rejected a self-releasing item as a hand-written wait in a transient file, and the human already judges `deferred` conditions in triage. The body's version makes the check mechanical for whoever reads it.
- **A `clm triage` line for every proposed change on `main`.** It shows that a change waits, but not on what, and adds a form for a case that a backlog item covers.
- **Put the contract step's design in the item, with no second change.** The old path's `clm:remove-by` marker would have no live change to name.
