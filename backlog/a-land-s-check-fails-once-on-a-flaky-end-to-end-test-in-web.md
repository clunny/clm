---
title: "A land's check fails once on a flaky end-to-end test in web"
kind: bug
components: [web]
---

Step: `clm land` and `clm land --check-only`, in an unattended `clm run` build of a component that touches nothing under web/.
What happened: the check script's web end-to-end suite failed on one test (the live page's hidden-tab case). Re-running the same check with nothing changed passed. The build lost a full check run, and an unattended build could just as well have stopped on it.
Expected: a check that fails only when the tree is broken, so a land's failure means something is wrong with the branch.
Rule involved: land-locally (every land runs the checks on this machine); a flaky check makes landing depend on retries.
