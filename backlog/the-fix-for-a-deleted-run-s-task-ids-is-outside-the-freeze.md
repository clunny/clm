---
title: "The fix for a deleted run's task IDs is outside the freeze"
kind: question
components: [orchestrator.loop]
blocks: [orchestrator.loop/launched-refuses-an-earlier-task, orchestrator.loop/returned-ignores-an-earlier-task]
choices:
  - "Build it now, as the late-report one"
  - "Wait for the plan's end"
---

`20261117-a-deleted-runs-task` has the build loop keep a finished run's task IDs by run name, so that a run `clm build start`, `clm build release` or a step drops still leaves them behind. Without it, the exception the human made on 2026-11-17 for `launched-refuses-an-earlier-task` and `returned-ignores-an-earlier-task` holds only while the earlier run stays listed: after a new session's `clm build start`, which drops every returned run, a component built again under the same name takes the first build's late report as its own, as before that exception.

The fix is not the plan's work (`backlog/a-passing-check-proves-less-than-the-spec-claims`), so whether the 2026-11-17 exception covers it is the human's to say. Answer "Build it now" to delete this item, so `orchestrator.loop`'s next build builds the two requirements at their new versions; answer "Wait" to have the plan item block them in this item's place, until the plan is done.
