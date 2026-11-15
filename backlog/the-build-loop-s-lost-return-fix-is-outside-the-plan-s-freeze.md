---
title: "The build loop's fix for a lost return is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
origin: 20261115-the-build-loop-waits
blocks: [orchestrator.loop/claim-gone-ends-a-build, orchestrator.loop/loop-start, orchestrator.loop/return-rules, orchestrator.loop/build-wait]
choices: ["Build it now", "Wait for the plan to end"]
---
May the change `20261115-the-build-loop-waits` be built while the plan's freeze is on?

The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says that until it is done the build loop and triage work only on it, that a defect's default answer is "accept", and that an exception is the human's to make, never a session's. The change fixes a defect that isn't the plan's work: a build loop waited 50 minutes on a build that had landed, because the subagent's report never reached the session, and then couldn't end a drain, because it kept a run of an earlier loop that `clm run` refused to relaunch. Its design is on `main`; this item holds its four requirements back until you answer.

- **Build it now** makes the exception, as you did for landing: every phase of the plan is built through this loop. Delete this item, and the loop builds `orchestrator.loop` next.
- **Wait for the plan to end** keeps the freeze. Leave this item until the plan is done, then delete it. Until then the two steps by hand are: report `landed <sha>` yourself with `clm build returned` for a build whose branch has merged, and run `clm build release <component>` for a run that every step prints as `skipped`.

Deleting this item is a spec change of its own, since no change lists it in `resolves`.
