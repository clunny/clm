---
status: applied
resolves: [a-deferred-item-that-blocks-a-requirement-is-never-taken-by-the]
---

## Intent

The build loop's human queue lists each `kind: deferred` item on `origin/main` that blocks requirements, with what it blocks and `judge with /clm:triage`, as it already lists the `question` and `human-task` items that do. A requirement a deferred item blocks waits on the human's judgement of that item, which no run of the loop takes, so the loop now says so instead of draining with the requirement silently held. The loop still never drafts a deferred item on its own, and `clm check` still accepts a deferred item with `blocks`. The one requirement it modifies, `orchestrator.loop/humans-queue`, is already held back by `backlog/a-passing-check-proves-less-than-the-spec-claims` for the plan's freeze, so its build waits for the human's word as the loop's other pending requirements do; no new item is needed for that.

## Problem

A wave's draft ordered four moves of code across components by `kind: deferred` items whose `blocks` named the requirements that had to wait for them. A builder check found that the build loop never takes a deferred item, so under `/clm:build` those requirements would have stayed blocked, and a component whose only pending requirements waited on them would never have been offered. The moves were made by the session before the wave instead.

Read against the design as it stands, the loop's not taking the item is right, and what is wrong is that nothing tells the human:

- A deferred item's condition is the human's to judge (`deferred-work-resolves-in-code`, `triage-owns-backlog-work`): `clm triage` prints it as a `deferred` line, and it is drafted only when the human says so. `/clm:build` drafts bug groups alone (`build-drafts-bugs`).
- A deferred item with `blocks` is a designed route, not a misuse: `answered-question-waits-as-deferred` turns an answered question whose work waits into a deferred item that keeps its `blocks`, so the requirements wait for the answer's work.
- `humans-queue` lists the `question` and `human-task` items that block requirements, but not the deferred ones. So a requirement held by a deferred item is in neither the loop's work nor the human's queue, and a drain ends with it held and no word of why.

Ordering a build after code work by a deferred item is the wrong tool where the work can be done first, as the session did: `a-wait-names-a-line` already says a step a build must take is stated in a requirement or made by a session that knows it, and `dependency-work-is-a-requirement` makes work a requirement relies on a requirement. With this change, such an item shows in the human's queue at the loop's first step, where it is judged at once.

No change or code fix since 2026-11-14 fixes it: the loop's changes of that time (`20261116-the-build-loop-shows`, `20261117-a-deleted-runs-task`, `20261117-the-build-loop-drafts` and those archived) change the queue's sections, task IDs and the draft's wait for its build, and none reads a deferred item.

## Approaches

- **Taken: the human's queue lists a deferred item that blocks requirements.** It is the existing rule for `question` and `human-task` items with one more kind (`reuse-first`): the step's read of `origin/main`'s backlog, which keeps those items with their kind, `blocks` and the time of the commit that added them, keeps a `deferred` item with `blocks` too, and the queue's filter takes one more kind with its own action word. The action is `judge with /clm:triage`, the command that offers the item as a `deferred` line, as a red verify result says `take in with /clm:triage`. The drain's end summary carries the human's queue as it stands (`end-summary`), so it names the item too.
- **Not taken: the loop takes a deferred item that blocks a pending requirement, as it takes a build.** It would draft work whose condition only the human judges, against `deferred-work-resolves-in-code` and `triage-owns-backlog-work`, and an answered question turned deferred would be carried out before the condition its answer names. It would also need `clm triage` to print such items as `group` lines, a second rule for one kind.
- **Not taken: `clm check` refuses a requirement blocked only by an item the loop never takes.** It would refuse the route `answered-question-waits-as-deferred` sets, and every `question` or `human-task` item with `blocks`, since the loop takes none of those either; what they need is the human, and the queue is where the loop asks for the human.

## Outcome

- `orchestrator.loop/humans-queue` lists each `deferred` item on `origin/main` whose `blocks` isn't empty, with what it blocks, `judge with /clm:triage`, and the time of the commit that added it, among the queue's other entries oldest first. A deferred item that blocks nothing isn't listed.
- `clm view queue` and the drain's summary show it, since both read the plan's human queue; `view` changes nothing.
- The decision `a-blocking-deferred-item-is-in-the-loop-s-queue` records the choice and refines `answered-question-waits-as-deferred`, whose answered question turned deferred keeps its `blocks` and so shows in the loop's queue under `judge with /clm:triage`, apart from the items waiting for an answer; `clm status` and the other views still list only `question` and `human-task` items as waiting on the human.
- The build reads `deferred` items with `blocks` in the step's backlog read (`crates/clm-loop/src/step.rs`'s `read_backlog` keeps only `question` and `human-task` items and those with `verify_failures` today), so a test through `clm build step` proves the scenario. `orchestrator.loop/humans-queue` also still waits on `orchestrator.loop/pause`'s paused-run entry, which its current text already states; both are held by the plan's freeze.

Edge cases and workarounds: none known.
