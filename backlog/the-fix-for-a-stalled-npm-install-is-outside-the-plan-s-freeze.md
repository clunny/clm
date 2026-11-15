---
title: "The fix for a stalled npm install is outside the plan's freeze"
kind: question
components: [ci]
origin: 20261115-a-stalled-npm-install
blocks: [ci/npm-installs-bounded]
choices: ["Build it now", "Wait for the plan to end"]
---
May the change `20261115-a-stalled-npm-install` be built while the plan's freeze is on?

The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says that until it is done the build loop and triage work only on it, that a defect's default answer is "accept", and that an exception is the human's to make, never a session's. The change fixes a defect that isn't the plan's work: an `npm ci` in verify's coverage pass sat for over 11 minutes on a stalled connection until you killed it by hand, and nothing else would have ended it. Its design is on `main`: the three check scripts run each npm install under a 300-second limit and try a stalled one once more. This item holds its one requirement back until you answer.

- **Build it now** makes the exception: the build loop waits on verify, and the same stall in a land's check would hold every land of the clone. Delete this item, and the loop builds `ci` next.
- **Wait for the plan to end** keeps the freeze. Leave this item until the plan is done, then delete it. Until then the step by hand is the one you took: when the machine sits idle during a verify or a land, find the `npm` process with no CPU time and kill it; the verify or land fails and is run again.

Deleting this item is a spec change of its own, since no change lists it in `resolves`.
