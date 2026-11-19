---
title: "The fix for a red verify lost to a note is outside the plan's freeze"
kind: question
components: [orchestrator.loop]
origin: 20261118-the-build-loop-loses
blocks: [orchestrator.loop/plan-verify]
choices: ["Build it now", "Wait for the plan to end"]
---
May the change `20261118-the-build-loop-loses` be built while the plan's freeze is on?

The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says that until it is done the build loop and triage work only on it, that a defect's default answer is "accept", and that an exception is the human's to make, never a session's. The change fixes a defect that isn't the plan's work: when `clm verify --due` fails under `/clm:build`, its last line of output is a note on stderr (`clm verify: output is at <log>`), not its `verify failed` line, so the session reports the note, and the loop records no red result and never puts it in your queue. Its design is on `main`: `plan-verify` now says the session reports the line of the output that starts `verify `, and `clm build returned verify` refuses any other line but `none`, so a wrong report fails loudly instead of losing the result. This item holds that requirement back until you answer. The build skill's sentence, `backlog/the-build-skill-reports-a-verify-run-s-last-line-not-its-outcome`, is a `deferred` item on `plugin` you can judge due on its own: it fixes the lost result in practice with the loop as built now.

- **Build it now** makes the exception: a red verify is how a slow test's break reaches you, and the mutation pass, phase 3's evidence, runs in it. Delete this item, and the loop builds `orchestrator.loop` next.
- **Wait for the plan to end** keeps the freeze. Leave this item until the plan is done, then delete it. Until then, after a verify under `/clm:build`, have the session report the line of its output that starts `verify `, as the deferred item's sentence will say.

Deleting this item is a spec change of its own, since no change lists it in `resolves`.
