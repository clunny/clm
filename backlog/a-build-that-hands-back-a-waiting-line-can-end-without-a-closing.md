---
title: "A build that hands back a waiting line can end without a closing line after it lands"
kind: bug
components: [plugin, orchestrator.run]
---

Step: /clm:build, handling a build subagent's return.

What happened: a build subagent handed back a report ending in a `waiting: <command>` line (its own reviews still running), then finished with no further hand-back. It never sent a closing `landed <sha>` or `stopped:` line, although its `clm land` had in fact run and its branch was merged into origin/main.

What the skill says: a subagent that ends without either closing line counts as a stop and is built again. Followed literally, the session would treat a landed build as stopped, tell the human it stopped and relaunch it. The skill never says to check whether the component's branch reached origin/main before treating a no-closing-line return as a stop. The session had to work that out itself, by looking for origin/impl/<component> and for a merge commit on origin/main.

Expected: one of two fixes. The build prompt could require a fresh closing line in the hand-back after any `waiting:` hand-back, once the awaited work ends. Or the build skill could say to read origin first (branch gone and merged into main means landed) before counting a return with no closing line as a stop.

Rules involved: background-work-stopped, builds-in-subagents.
