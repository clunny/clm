---
title: "The fix for a printed run shown as running is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
origin: 20261116-the-build-loop-shows
blocks: [orchestrator.loop/pause, orchestrator.loop/queue-tells-printed-from-running, orchestrator.loop/loop-file, orchestrator.loop/humans-queue]
choices: ["Build it now", "Wait for the plan to end"]
---
May the change `20261116-the-build-loop-shows` be built while the plan's freeze is on?

The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says that until it is done the build loop and triage work only on it, that a defect's default answer is "accept", and that an exception is the human's to make, never a session's. The change fixes a defect that isn't the plan's work: after you asked a `/clm:build` session to hold a verify that hangs, the loop printed `run verify ...` at every step, and `clm view queue` and the panel listed it under running for 48 minutes with nothing behind it. Its design is on `main`: `clm build pause <name>` records your word that a printed run waits, `clm build pause --end <name>` your word to go on, a paused run waits in your queue, and the queue lists a run the session hasn't launched under next up as `printed, not launched yet` instead of under running. This item holds its four requirements back until you answer.

- **Build it now** makes the exception: the loop builds every remaining phase, and a pause is how you keep it off a step known to hang without stopping it. Delete this item, and the loop builds `orchestrator.loop` next.
- **Wait for the plan to end** keeps the freeze. As for the npm fix, the plan's item records the answer and takes over this item's `blocks`, and this item is deleted (`answered-question-waits-as-deferred`); when the plan is done, the four requirements come out of its `blocks` and the loop builds them. Until then, when you want a printed run held, have the session leave its action alone, and read a `running` entry with no subagent or task behind it in `clm view queue` as not started.

Deleting this item is a spec change of its own, since no change lists it in `resolves`.
