---
title: "The build loop shows a printed run action as running, and gives no way to defer it"
kind: bug
components: [orchestrator.loop, view]
---

What happened: in a /clm:build session, the session stopped a hung `clm verify --due` at the human's request and reported it with `clm build returned verify ... --cause stopped`. Every later `clm build returned`, `step` and `approved` call printed `run verify clm verify --due --event work-done` again. The human asked to hold the verify until a fix for the hang had landed, so the session never ran it nor called `clm build launched` for it. `clm view queue` and the CLM panel's Queue tab then listed it under "running" (`verify  round 1  48m`) with no process or subagent behind it, and the human asked whether a verify was running and why the panel showed it.

Expected: a run the loop has printed but the session hasn't launched is shown apart from a live run (for example as "to launch" or "held"), and the session has a recorded way to defer or decline a printed `run`/`launch` action on the human's word, such as a hold until a condition or until a change lands, instead of a choice between running a step known to hang and leaving a stale "running" entry.

Rule involved: the build skill's "Never decide a launch, an order, a retry or a resume yourself", which leaves the human's "hold it" with no command to carry it.
