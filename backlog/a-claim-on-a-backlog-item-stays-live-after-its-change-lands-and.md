---
title: "A claim on a backlog item stays live after its change lands and keeps the item"
kind: bug
components: [orchestrator]
origin: 20261114-proof-is-real
---

Step: a draft claims a backlog item with `clm claim backlog/<slug>`, and its change lands while leaving the item in the backlog (it narrows the item, or defers it).

What happened: the `propose/<slug>` branch stayed on `origin`, so `clm proposals` and `clm status` showed the item as claimed for four more hours, and a later session that touched the same components had to ask the human whether a dead claim still held them. Only the idea-elaboration steps of the propose skill say to delete such a claim; a change that keeps its item says nothing.

Expected: landing a change releases the claims its session took on items the change leaves in the backlog, or `clm wrap-up` does.

Rule involved: `propose-claims`.
