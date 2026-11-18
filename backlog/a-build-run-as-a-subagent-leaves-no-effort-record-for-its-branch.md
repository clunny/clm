---
title: "A build run as a subagent leaves no effort record for its branch"
kind: bug
components: [effort]
---

Step: `/clm:build` ran one component's build as a subagent of the session, in a worktree of its own, and `clm land impl/<component>` landed it.

What happened: the ledger holds the `landed` record and no `effort` record for `impl/<component>` from that build. `clm effort record --branch impl/<component>` afterwards printed `nothing new to record`. The build's three transcripts (the builder and its two reviewers) are on this machine under the launching session's `subagents/` directory; every entry of them carries the branch name `main`, the branch of the session that launched them, though the work was done on `impl/<component>` in the subagent's worktree. Effort is attributed by the branch an entry names, so the build's work counts toward `main` or nothing.

Expected: the model work a build spent is recorded against its `impl/<component>` branch, so a capability's cost includes its builds (`effort-accounted`, `land/records-effort`, `effort/attribution`).

Why it matters now: the plan in force wanted the cost of one component's pass read from this record, and it had to be added up by hand from the transcripts. Earlier `effort impl/*` records of a few thousand tokens for whole builds suggest the same cause.
