---
title: "The fix for a landed build's task reported for the next build of its component is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
blocks: [orchestrator.loop/launched-refuses-an-earlier-task, orchestrator.loop/returned-ignores-an-earlier-task]
choices:
  - "Build it now, as an exception"
  - "Wait for the plan's end"
---

`20261117-clm-build-step-prints` has `clm build launched` refuse a task ID the loop file records for another run, or for an earlier run of the same name, such as the subagent of a build that landed before it reported, whose component the loop then builds again under the same name. Its stderr says the printed `launch` is a new run to start as a new subagent. Without it, a session that takes that `launch` for a duplicate and reports the old task ID leaves the new build's claim held with nothing building it, until someone notices. `20261117-a-late-report-of` adds `returned-ignores-an-earlier-task`, the same fix for a return: `clm build returned --task` with a task ID the loop keeps as an earlier run's of the name changes nothing, so a late `none --cause context` of the landed build's subagent no longer finishes and rebuilds the new build. It reads the task IDs `launched-refuses-an-earlier-task` keeps, so the two are built together or not at all. Neither is the work of `backlog/a-passing-check-proves-less-than-the-spec-claims`, whose freeze holds for everything the human doesn't except. Should `orchestrator.loop`'s next build build `launched-refuses-an-earlier-task` and `returned-ignores-an-earlier-task` now, or should it wait for the plan's end, this item then blocking them as the plan item does `pause`?
