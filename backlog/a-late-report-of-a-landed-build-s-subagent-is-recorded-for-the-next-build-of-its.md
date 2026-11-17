---
title: "A late report of a landed build's subagent is recorded for the next build of its component"
kind: bug
components: [orchestrator.loop]
---

Found while drafting `20261117-clm-build-step-prints`. When a build lands before its subagent reports, `claim-gone-ends-a-build` finishes the run, and the plan may launch the component again under the same name, `build:<component>`. `clm build returned` names a run only by its name. `return-rules` ignores the old subagent's late `landed <sha>`, by the commit's time, but any other line from it is recorded for the new run. A `none --cause context`, for example, would finish the new build and rebuild it, and a `none --cause work` would count a stop against it.

Expected: a return from a run the loop has already finished changes nothing of the run now listed under its name. No requirement states this beyond the `landed` case. `launched-refuses-an-earlier-task` keeps each name's earlier task IDs, so a fix could name the task in `returned`, or the session could be told which task a return came from. Rule: `return-rules`, `claim-gone-ends-a-build`.
