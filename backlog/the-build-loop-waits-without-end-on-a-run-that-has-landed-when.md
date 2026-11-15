---
title: "The build loop waits without end on a run that has landed when the subagent's return never arrives"
kind: bug
components: [orchestrator.loop]
---

Step: `/clm:build`, carrying out `wait` actions from `clm build step` while one launched build run was outstanding.

What happened: the run's subagent landed its branch on `main` (its `impl/*` branch merged and gone from `origin`, its worktree removed), but the agent harness never delivered the run's final report to the session. The session had no last line to pass to `clm build returned`, so the loop kept listing the run as running. `clm build wait` returned every few minutes and `clm build step` printed only `wait` again, for about 50 minutes, until the session looked at `origin/main` itself and reported `landed <sha>` from the merge commit.

Expected: `clm build step` notices that a running build's `impl/<component>` branch has merged into `origin/main` and treats the run as returned with `landed <sha>`, or at least prints an action (such as a `say`) telling the session the run looks finished. The skill gives the session no instruction for a run whose return never arrives; it only says to report "every subagent's return".

Also seen in the same session: `clm view queue` kept listing a run under `running` (`round 1`, hours old) that every `clm build` command reported as `skipped` because its component had nothing pending.

Rules involved: `build-loop-in-clm`, `builds-in-subagents`.

Later in the same session: once the landed run was reported, that skipped run was the only one left under `running`, and the loop could not end. Every `clm build step` printed the `skipped` notice for it and then `wait`. This session never received a `launch` action for it, so the agent harness held no subagent to return. Reporting it with `clm build returned <name> --result none --cause tool` changed nothing: the run stayed under `running` at `round 1` and the next action was `wait` again. Expected: a run the loop itself skips leaves `running`, and a loop with nothing running and nothing ready prints `end`.

Cause of the stuck run, read from the loop's code: `clm build start` marks every run a previous loop left unreturned as an orphan to relaunch. Relaunching a build orphan runs `clm run <component>`; when that refuses with exit 1 because the component has nothing pending, the step prints `skipped` and keeps the orphan as it was. A step counts any orphan as busy, so the loop is never idle: it relaunches and skips the same orphan on every step. Expected: an orphan whose relaunch is refused because its component has converged is dropped.
