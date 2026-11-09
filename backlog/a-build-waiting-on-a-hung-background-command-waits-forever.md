---
title: "A build waiting on a hung background command waits forever"
kind: bug
components: [orchestrator.run, plugin]
---

During /clm:build, a build subagent started its component's test run as a background command, twice. The test hung (a polling wait with no deadline), so both runs never ended. The build subagent then reported "waiting on its own background work", and the build skill's rule says such a build has not returned: don't finish it, don't retry it, count it as running. Nothing in the process ever noticed the hang. The build sat idle for about 80 minutes until the human asked why it was slow; the orchestrating session then had to inspect processes by hand, kill the hung test processes on the human's yes, and message the build to resume.

Expected: a build's own test and check runs have a deadline (the build prompt asks for a timeout on test commands, or for running them in the foreground), and/or the build skill tells the orchestrating session to check on a build that has waited on background work beyond some bound, and to report it to the human, rather than waiting indefinitely.

Rules involved: background-work-stopped (the waiting rule in the build skill) and the build prompt that clm run prints.
