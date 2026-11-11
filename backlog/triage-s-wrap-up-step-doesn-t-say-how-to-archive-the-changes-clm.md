---
title: "Triage's wrap-up step doesn't say how to archive the changes clm wrap-up lists as complete"
kind: bug
components: [plugin.triage]
---

Step: `/clm:triage` step 10, running `clm wrap-up` at the end of a triage session.

What happened: `clm wrap-up` printed `complete <id>` for a backlog-and-decision change, with no deltas, that a draft had landed during the session. The triage skill says only "fix what it reports and run it again". It doesn't say what to do with a `complete` line, so the session had to find a procedure. It tried `clm archive <id>`, which refused the change because its status was `applied`. The session then set `status: done` by hand before archiving, instead of using the route `/clm:build` step 6 states: an `impl/archive/<id>` branch where `clm rebase` archives every complete change and deletes the items they resolve.

Expected: the triage skill's wrap-up step handles `complete <id>` lines the way the build skill's does, or points to that step. Either way, a triage session archives through `clm rebase` on `impl/archive/<id>`.

Rule involved: `checkpoint-before-done`, `archive-branches` and `archive-on-sync`. The `plugin.triage` skill text has no requirement for the step, so the fix probably needs a spec change.
