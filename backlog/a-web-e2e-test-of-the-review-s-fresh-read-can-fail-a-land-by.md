---
title: "A web e2e test of the review's fresh read can fail a land by reading Cache Storage before the copy is written"
kind: bug
components: [web]
---

Step: `clm land` of a component build branch (the land check runs the web suites).

What happened: the land check failed on a web end-to-end test of the approve flow ("approve: the review is requested again with a UUID…"), a test the branch had not touched. Run again with nothing changed, the same land passed. The builder's reading: the test reads the browser's Cache Storage as soon as it sees the reload's request, before the app has written the copy it asserts on, so whether it passes depends on timing.

Expected: a land check that fails only when the tree is wrong; a test that waits for the state it asserts.

Rule involved: land checks gate every land of the clone (`land-locally`); a check that fails by timing costs a full re-run and can block unrelated branches.
