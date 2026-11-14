---
title: "The unit of building is the node: its code is brought up to its whole spec, and only a requirement's Needs orders work"
date: 2026-10-23
status: accepted
---

A change is the unit of intent: a human approves it, and `clm apply` folds its text into `specs/` (`apply-in-spec-pr`). Before this decision the change was also the unit of building: one plan per change, one task per change per node, a `finish` item, and tasks that recorded the bundle hash they were built on. But `specs/` on `main` holds every applied change's text for a node at once, so a task on a node either built other changes' text by accident or had to be told, change by change, which text was its own. Every ordering rule CLM grew came from reconciling those two units: `after` and `prefer_after` between changes, the demand that every overlap be ordered, cross-change waits derived from shared nodes, code-order waits within and across changes, plan staleness and re-plans, `gate/task-done-fresh`, and priority inheritance. Each was right locally. Together they fixed order at propose time from arrival rather than need, held work for weeks behind changes that didn't need it, re-planned a change after almost every other change that touched its nodes, and failed finished PRs because `main` had moved under them.

So the unit of building is the node. A build session brings a node's code up to the spec on `main`, all of it, highest priority first. The queue is derived each time it is read from three things that are all committed: the node graph, the test markers with their versions (`versioned-markers`), and the backlog items that block requirements (`requirement-blocks`).

- **Only `Needs` orders work.** A node is ready when it has pending work no item blocks, whatever the state of the nodes it depends on: only a requirement's `Needs` holds work back (`needs-alone-order`, `requirement-needs`). A cycle of needs is a loading error, so nothing waits for itself, and no wait between changes exists to point the wrong way.
- **Nothing is planned.** The queue reads no `plan.md` or task file, and there is no `finish`. (A change run by hand may keep a `plan.md` as notes, as the re-founding does; nothing parses it.) What is left to build is read from the markers, so a merged PR is progress whatever it leaves, and a spec that moves under a build adds pending text instead of invalidating work.
- **Every ready node builds in parallel.** Each runs in its own worktree on `impl/<node>`, claimed by lease. Node `paths` are disjoint, so two sessions edit the same file only under `path-ownership`'s exceptions; a change another node must follow is built in steps, each in its own node's files (`coupled-changes-sequence`). A node too hot to keep up is a reason to split it in the architecture, which is pressure in the right direction.
- **Changes carry priority and progress.** A pending requirement takes the priority of the live change that added, modified or removed it (`priority-orders-nodes`). A change is complete when its requirements are current and nothing cites one it removed, and `clm sync` archives it (`archive-on-sync`).
- **A build is scoped to a node, whatever intent its text came from.** One PR may carry text from several changes. Humans review intent in spec PRs, not code (`humans-review-intent`), so the mixing costs no review. Progress per change is read from its requirements.

Refines `priority-caps` (the caps stay; nothing orders one change against another), `blocked-by-backlog` and `task-deletes-fixed-items` (items block requirements, and the build that fixes an item deletes it), and `lease-claims` and `merged-claims-released` (the claim is `impl/<node>`).

Rejected:
- **Keeping change-scoped tasks and pruning the waits** to requirement-level dependencies declared by the author. Smaller, but a task still builds other changes' text by accident, so staleness and re-plans stay, and the author has to know the code's wiring.
- **A task per requirement.** Finer than the paths that keep parallel sessions apart, and many more PRs for the same code.
- **Design-only changes making a node pending.** A node is pending on requirements only; a Design edit alone builds nothing, as `archive-in-spec-pr` already treats it. Reopen if Design edits that should drive refactors turn out common.
