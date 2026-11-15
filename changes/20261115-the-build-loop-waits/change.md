---
status: proposed
resolves: [the-build-loop-waits-without-end-on-a-run-that-has-landed-when]
---

## Intent

A build loop must end. Today it waits without end on a run whose branch has landed when the subagent's return never reaches the session, and on a run a previous loop left that its own relaunch refuses because the component has nothing pending.
