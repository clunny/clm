---
title: "Defects a build-loop session reports never reach main"
kind: bug
components: [orchestrator.loop]
---

Step: the /clm:build skill tells the build-loop session to report each CLM defect it meets with `clm defect`, and says that, since it opens no pull request of its own, "the loop drafts reported defects with the backlog's other bugs".
What happened: `clm defect` run in the build-loop session wrote each report as an uncommitted file in the main checkout's backlog/. Nothing committed or landed them, so they never reached main. The loop plans from main, so it never drafted them; it reported nothing next and, when stopped, said nothing unsafe was left, while three reports sat untracked in the checkout. One of them was the cause of every failed verify that session.
Expected: a defect the build-loop session reports reaches main (the loop lands it, or the skill gives the session a route to land it), or the loop's end-of-loop summary names reports left uncommitted.
Rule: defect-reports, and the build skill's statement about how its reports get drafted.
