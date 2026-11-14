---
title: "A draft can be finished while a command it started still runs"
kind: idea
components: [orchestrator.triage]
---

`clm draft --finish <id>` frees the draft's place and removes its worktree. The triage skill tells the session not to finish a draft whose result ends `waiting: <command>`, and the draft prompt tells the draft not to write its final message while its `clm land` runs. The requirements that pinned those sentences (`plugin.triage/triage-waiting-drafts`, `orchestrator.triage/background-work-stopped`) were scans of prompt text and are deleted in `20261113-phase-2-delete`.

If it must hold, `clm draft --finish` is the program that can hold it: refuse while a land or check started in that worktree is still running.
