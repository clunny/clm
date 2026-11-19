---
title: "An obsolete requirement's marker in another component's file keeps its change from completing"
kind: question
components: [orchestrator.run, check.coverage]
---

Once the root sets `wiring`, `orchestrator.run/pending-prompt` lists an `obsolete` requirement's marker in a file another component owns apart, and tells the build to leave it and add a `question` item that blocks nothing. Nothing then removes it: the owner's `clm run` lists only the owner's own requirements (`pending_in` keeps `p.node == node`), so the owner's build never sees the removed requirement either. While the marker stays, the requirement stays `obsolete`, so the removing change never completes and `archive-on-sync` never archives it, and `clm pending` of the removing component reports it after every build.

A related loss: when an approved `deleted-tests.txt` names such a test, the prompt says to leave it and doesn't say a human already approved its deletion, so the approval can be lost in the `question` item.

What should move the marker: should the owner's build list obsolete markers of other components' requirements found in its own files (with the approved deletions), or should the removing change's archive wait on a step of the owner's? Raised by the review of the `impl/orchestrator.run` build that added the listing apart.
