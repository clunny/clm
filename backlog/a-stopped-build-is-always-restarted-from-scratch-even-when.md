---
title: "A stopped build is always restarted from scratch, even when resuming its agent would serve better"
kind: bug
components: [plugin]
---

Step: /clm:build, step 3, handling a build subagent that returned `stopped: <reason>`.

What happened: several builds stopped only because the disk filled during `clm land` (ENOSPC in the land check). Their code was done and reviewed; only the land needed re-running once space was freed. The build skill allows exactly one exception to "build it again with a new subagent": a usage or rate limit, which resumes the same subagent with SendMessage (`resume-limited-builds`). Every other stop must be retried as a fresh `clm run` build in a new worktree. So the orchestrator relaunched them fresh: each retry loses the stopped agent's context (what it tried, review findings it weighed), recompiles the workspace from an empty target/ (several GB each, on a machine already short of disk), and may fail at once on `git switch impl/<component>` because the stopped build's worktree, which `clm run --finish` keeps when the branch hasn't landed, still has that branch checked out. Uncommitted files the stopped agent left (a drafted PR body) are also lost.

Expected: whether to resume the stopped subagent (SendMessage, same worktree and context) or start a fresh build should be the orchestrating session's judgement, based on why the build stopped. A rate limit is one case for resuming; an environmental failure such as a full disk, a network outage or a transient tool failure, where the work itself is sound, is another; a build that went wrong in its approach, ran out of context, or stopped twice for the same reason is a case for a fresh start. The skill should state that judgement and its criteria rather than enumerating rate limits as the only resumable stop.

Rule involved: `resume-limited-builds`; the retry paragraph of /clm:build step 3; `clm run --finish` keeping an unlanded build's worktree.
