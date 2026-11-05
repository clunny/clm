---
title: "clm run can't resume a claim its own interrupted run pushed"
kind: bug
components: [orchestrator.run, orchestrator]
issues: [148]
---

If `clm run <component>` is interrupted after it has pushed the claim commit to `origin/impl/<component>` but before it has made the local `impl/<component>` branch, the clone keeps no local trace of the claim. Every later `clm run <component>` refuses because the component is claimed, and `clm run --finish` doesn't clear that. The only ways on are deleting the remote claim branch, which auto mode blocks as destructive, or waiting out the four-hour expiry.

Expected: `clm run` picks up a claim the same clone just pushed when the claim holds nothing beyond its commit, or records the claim locally in a way an interruption can't split from the push. Either way, an interrupted launch recovers without deleting a remote branch, and another clone still can't take a live claim (`lease-claims`).
