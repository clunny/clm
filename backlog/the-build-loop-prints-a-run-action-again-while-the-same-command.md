---
title: "The build loop prints a run action again while the same command is still running"
kind: bug
components: [plugin]
---

Step: the /clm:build skill carrying out `clm build` actions.

What happened: `clm build returned` for one run printed `run verify clm verify --due`; the session started it as a background command. When the next run returned, `clm build returned` printed the same `run verify clm verify --due` line again while the first command was still running. A `launch` has `clm build launched` to record that it started, but a `run` action has nothing like it, so the loop can't tell the command is already running and repeats the action on every step until it returns.

Expected: the loop knows a `run` action it printed is in flight (by recording it when printed, or through a `clm build launched`-style acknowledgement) and doesn't print it again until it returns. Otherwise the skill has to choose between running a duplicate copy of a slow command and not carrying out an action the loop printed, against "never decide a launch ... yourself" (`build-loop-in-clm`).

Checked on 2026-11-15 (`20261115-clm-build-returned-fails`): the fix is code-only and lies in `plugin`. The loop already does what is expected: `orchestrator.loop/step-actions` says a run still `planned` is printed again until `clm build launched` reports it, a `run` action is such a run (`verify`, `land:<batch>`), and `clm build launched <name> <task-id>` records any run as running, after which no step prints it again. What is missing is the acknowledgement in `plugin/skills/build/SKILL.md`, whose `run` bullet doesn't say to run `clm build launched <name> <task-id>` with the background command's task ID, as its `launch` bullet does. Recording a `run` as running when it is printed was not taken: a line lost to compaction would then never be printed again, which is what the acknowledgement guards against for a `launch`.
