---
title: "Releasing a dead session's claim leaves the build loop skipping the component"
kind: bug
components: [plugin, orchestrator.loop]
---

Step: the build skill says to release a component's claim by deleting `impl/<component>` on `origin` with `clm branch delete`.

What happened: after the delete succeeded, `clm build step` skipped the component, because the clone still held a local `impl/<component>` branch, checked out in the dead session's worktree. The skip message says to reconcile or delete the local branch, but the skill states no such step, and the session had to move the local branch out of `impl/` by hand before the loop would launch the build.

Expected: the skill (or `clm branch delete`, or the loop) says what to do with the local branch a dead session leaves behind in the same clone, so releasing a claim is one stated procedure.

Rule involved: `build-loop-in-clm` (the loop decides every launch; the session should need no unstated manual step).
