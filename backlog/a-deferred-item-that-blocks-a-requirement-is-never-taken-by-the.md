---
title: "A deferred item that blocks a requirement is never taken by the build loop"
kind: bug
components: [orchestrator.loop]
---

A requirement's build can be ordered after a code-only fix only by a `kind: deferred` backlog item whose `blocks` names the requirement. The build loop never schedules a deferred item: only `clm claim <component> --fix` in a session takes one. So a change whose builds wait on such items never becomes ready under `/clm:build`: the blocked requirements stay blocked, and a component whose only pending requirements Need them is never offered. Met when a wave's draft ordered four moves of code across components this way; a builder check found it before it landed, and the moves were made by the session before the wave instead. Expected: either the loop takes a deferred item that blocks a pending requirement, as it takes a build, or `clm check` refuses a change whose requirement is blocked only by an item the loop will never take (`requirement-blocks`, `a-wait-names-a-line`).
