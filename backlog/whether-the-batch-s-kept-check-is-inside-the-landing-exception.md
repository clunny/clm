---
title: "Whether the batch's kept check is inside the landing exception to the freeze"
kind: question
components: [land]
origin: 20261115-a-batch-whose-main
blocks: [land/batch-keeps-its-check]
choices: ["Build it now", "Wait for the plan to end"]
---
May `land/batch-keeps-its-check` be built while the plan's freeze is on?

The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`, Settled) says that until it is done the build loop and triage work only on it, that a defect's default answer is "accept", and that an exception is the human's to make, never a session's. You made one for landing (2026-11-13) and extended it to the lands taken together (2026-11-15).

The change `20261115-a-batch-whose-main` adds one requirement that finishes that work. `land`'s Design and the decision `batched-lands` say lands taken together keep their check when `main` moves under it and run again only what they and `main`'s new commits can both break. No requirement said so, and the build left it out: such a batch pushes nothing and each of its lands runs a check of its own. A land with nothing to check moves `main` under a batch's check on the same clone, so this happens in ordinary build sessions. The session that drafted the change read it as inside your exception, but left the call to you. Its design is on `main`; this item holds the requirement back until you answer.

- **Build it now** counts it as part of the landing exception. Delete this item, and the loop builds `land` next.
- **Wait for the plan to end** keeps the freeze. Leave this item until the plan is done, then delete it. Until then a batch whose `main` moved costs each of its lands its own check, as today.

Deleting this item is a spec change of its own, since no change lists it in `resolves`.
