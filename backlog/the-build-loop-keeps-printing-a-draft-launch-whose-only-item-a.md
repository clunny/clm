---
title: "The build loop keeps printing a draft launch whose only item a landed build has resolved"
kind: bug
components: [orchestrator.loop]
---

Step: the /clm:build loop (`clm build returned`, `clm build step`).

What happened: a build's review filed a kind: bug backlog item, and the loop printed a `launch draft:<id>` for a group holding only that item. The launch was held while a build of the same component ran; that build fixed the bug and deleted the item with a `Resolves:` trailer, and it landed. After that, every `clm build returned` and `clm build step` (with the local checkout fast-forwarded to origin/main, where the item no longer exists) still printed the same `launch draft:<id>`, so the loop could not drain. No `propose/*` claim branch existed on origin for it.

Expected: once every item a pending draft group names is gone from main, the loop drops the group instead of printing its launch, rather than spending a draft session whose prompt would only find the item gone.

Rule involved: `build-drafts-bugs`; the draft prompt's own "check each item against origin/main" step shows the case is known but is handled only after a session is spent.
