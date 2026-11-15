---
status: proposed
resolves:
  - clm-build-returned-fails-when-its-wrap-up-deletes-a-branch-a
  - the-build-loop-prints-a-run-action-again-while-the-same-command
---

## Intent

The build loop's return report survives a wrap-up that races a run's own cleanup, and the loop doesn't print a `run` action again while the command it printed is still running.
