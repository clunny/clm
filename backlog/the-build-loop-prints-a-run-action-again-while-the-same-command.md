---
title: "The build loop prints a run action again while the same command is still running"
kind: bug
components: [orchestrator.loop]
---

Step: the /clm:build skill carrying out `clm build` actions.

What happened: `clm build returned` for one run printed `run verify clm verify --due`; the session started it as a background command. When the next run returned, `clm build returned` printed the same `run verify clm verify --due` line again while the first command was still running. A `launch` has `clm build launched` to record that it started, but a `run` action has nothing like it, so the loop can't tell the command is already running and repeats the action on every step until it returns.

Expected: the loop knows a `run` action it printed is in flight (by recording it when printed, or through a `clm build launched`-style acknowledgement) and doesn't print it again until it returns. Otherwise the skill has to choose between running a duplicate copy of a slow command and not carrying out an action the loop printed, against "never decide a launch ... yourself" (`build-loop-in-clm`).
