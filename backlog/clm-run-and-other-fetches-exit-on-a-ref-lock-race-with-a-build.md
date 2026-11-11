---
title: "clm run and other fetches exit on a ref-lock race with a build in the same clone"
kind: bug
components: [orchestrator.run, orchestrator.checkpoint, orchestrator, orchestrator.triage]
---

During `/clm:build` with several builds running, `clm run <component>` exited 2 with `git fetch --prune --quiet origin: error: cannot lock ref 'refs/remotes/origin/impl/<other>': is at <sha> but expected <sha>`: a build in another worktree of the same clone moved that remote-tracking ref during the fetch. An immediate retry succeeded.

`clm_git::fetch_retrying` (`vcs`) was added for exactly this race, but only `clm land`, `clm reconnect` and `clm approve` use it. `clm-run`, `clm-checkpoint`, `clm-orchestrator` and `clm-triage`'s draft, finish and fold still run a plain `git fetch --prune --quiet origin`, so any of them can fail on a race that parallel builds make routine.

Expected: every fetch CLM runs against `origin` goes through `fetch_retrying`.
