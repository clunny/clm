---
title: "A deferred item that blocks requirements is in the build loop's human queue"
date: 2026-11-22
status: accepted
---

A `kind: deferred` item may carry `blocks`: `answered-question-waits-as-deferred` keeps them when an answered question's work waits, and a draft once used them to order builds after code moves. No run of the build loop takes a deferred item, since its condition is the human's to judge (`deferred-work-resolves-in-code`, `triage-owns-backlog-work`, `build-drafts-bugs`), and the loop's human queue listed only `question` and `human-task` items. So a requirement a deferred item blocked was in neither the loop's work nor the human's queue, and a drain ended with it held and no word of why.

Now `orchestrator.loop/humans-queue` lists each deferred item on `origin/main` whose `blocks` isn't empty, with what it blocks and `judge with /clm:triage`, the command that offers it as a `deferred` line. Its action says the human judges a condition, not that a question waits for an answer.

Refines `answered-question-waits-as-deferred`: an answered question turned deferred keeps its `blocks`, so it shows in the loop's queue under `judge with /clm:triage`, apart from the items waiting for an answer. `clm status` and the other views still list only `question` and `human-task` items as waiting on the human, as that decision says.

Rejected:
- **The loop drafts a deferred item that blocks a pending requirement.** It would carry out work whose condition only the human judges, and would need `clm triage` to print such items as `group` lines.
- **`clm check` refuses a requirement blocked only by an item the loop never takes.** It would refuse the route `answered-question-waits-as-deferred` sets, and every `question` or `human-task` item with `blocks`, which need the human, not a refusal.
