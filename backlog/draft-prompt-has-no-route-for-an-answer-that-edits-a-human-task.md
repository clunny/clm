---
title: "Draft prompt has no route for an answer that edits a human-task and leaves it open"
kind: bug
components: [orchestrator.triage]
---

Step: `clm draft --answers <file> <slug>` for a `human-task` item the human answered in `/clm:triage` with "trim the done steps out of the item and keep the rest open".

What happened: the draft prompt tells the session to write `resolves` naming every item, and says a `question` or `human-task` item the answers settle stays in `resolves`. A change that resolves an item deletes it when archived (`task-deletes-fixed-items`), so following the prompt would have deleted the steps the human wanted kept. The draft session had to deviate on its own judgment and write `resolves: []`.

Expected: the prompt names a route for an answer that only edits an item (partly done, still waiting on the human): edit the item's body on the spec branch, leave it out of `resolves`, and say so in `## Intent`, the same as the backlog-only route for dropped items. Triage's step 4 gives the human that choice, so the draft should be told how to carry it out.
