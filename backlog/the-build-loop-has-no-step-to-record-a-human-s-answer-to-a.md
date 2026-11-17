---
title: "The build loop has no step to record a human's answer to a question item"
kind: bug
components: [orchestrator.loop, plugin]
---

What happened: in a /clm:build session, a draft the loop launched filed a `question` item that blocks four requirements, and the human answered it in the session ("build it now, as an exception"). The build skill and `clm build` give no action for recording a human's answer: the loop drafts bugs, and nothing it prints takes an answered question in. The session borrowed the triage skill's route, writing the answer to `.clm/answers.md` and running `clm draft --answers .clm/answers.md <slug>`, then launched that draft outside the loop (no `clm build launched`) and ran `clm draft --finish` itself. `.clm/answers.md` still held two answers from an earlier triage session, which the session had to notice and remove so they would not reach this draft.

Expected: the build skill says how to record a human's answer to a `question` or `human-task` item that blocks the loop's work, and `clm build` has a command for it (for example `clm build answered <slug> <answer>`) that drafts the answered item inside the loop, with its launch, return and finish counted like the loop's other drafts.

Rule involved: the build skill's "What needs the human is in the loop's queue" and "Never decide a launch ... yourself"; the loop's queue showed the question as waiting on the human but offered no way to act on the answer.
