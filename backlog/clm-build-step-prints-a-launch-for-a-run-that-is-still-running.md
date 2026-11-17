---
title: "clm build step prints a launch for a run that is still running"
kind: bug
components: [orchestrator.loop]
---

Step: `/clm:build`. After a `clm build wait` woke, `clm build step` printed `launch build:<component>` for a component whose run had been launched and recorded with `clm build launched <name> <task-id>` about 15 minutes earlier. That run was still running in the agent harness, its claim was live (pushed under an hour ago), and it had not been reported with `clm build returned`. The loop also rewrote that run's launch prompt file.

Expected: the loop does not print a launch for a run it has recorded as launched and that has not returned. Carrying the action out would start a second build on the same `impl/*` branch. The session re-ran `clm build launched` with the same task ID instead, after which `step` printed `wait`. The skill gives no instruction for this case, and its rule "never decide a launch yourself" leaves the session either to duplicate the build or to step outside the skill.

Possibly related: the verify run in the same loop had just returned with `--cause tool`, and the wait woke on that; the relaunch came on the next step.

Rule: `build-loop-in-clm` (the loop decides every launch) and `builds-in-subagents`.
