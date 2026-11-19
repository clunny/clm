---
status: applied
---

## Intent

This change only adds a backlog item: `backlog/the-build-loop-loses-a-red-verify-reported-by-its-last-line`, a CLM defect the draft met.

The item it was drafted for, `the-build-skill-gives-no-cause-for-a-run-action-that-succeeds`, was fixed on `impl/plugin` (landed 4845174a): the fix lies only in the build skill's text (`prompt-text-is-source`), which now says to report a `run` action's last line with no `--cause`, however the command ended, since `clm build returned` reads a `verify` or `land:` run's outcome from its line and ignores any cause (`orchestrator.loop/plan-verify`, `crates/clm-loop/src/returned.rs`). So it leaves this change's `resolves`.

While checking that, the draft found that a failed `clm verify` ends its output with a stderr note, not its `verify failed` line, so a session that reports the run's last line, as `plan-verify` says, loses the red result from the loop's queue. Its fix needs a spec change to `plan-verify`, so it is filed for a later draft.
