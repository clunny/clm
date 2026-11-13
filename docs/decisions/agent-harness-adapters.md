---
title: "Every rule lives in clm and git; an agent harness adapter only recognises, translates and declares"
date: 2026-11-13
status: accepted
---

`ARCHITECTURE.md` stated this as the entry `agent-harness-adapters` until 2026-11-13, when the entry was cut to its rule (`human-layer-entries-stay-short`). The text still holds:

Every rule SHALL live in `clm` and git, never in an agent harness. An agent harness adapter only recognises its agent harness from the environment of the command it runs, translates the agent harness's hook events into `clm`'s and `clm`'s decisions back into the agent harness's format, and declares what the agent harness can do: make the human confirm a command, run a hook before a shell command, run hooks at a session's start and end, launch subagents, give a subagent a worktree of its own, and choose a subagent's model. Where a recognised agent harness lacks an ability, CLM degrades to a path that keeps every rule (the human's own terminal for an approval, builds one at a time where there are no subagents, the session's own model where it can't choose one), never to one that trusts the agent. An agent of the generic tier has no adapter, so CLM can't tell it from the human's terminal: there no guard runs and `clm approve` can't refuse, and what holds is what `approval-policy` and `guard-blocks-push-to-main` say under `#### Untested`.

**Scenario: an agent harness that can't confirm**
- GIVEN a session under a recognised agent harness whose adapter declares no confirmation
- WHEN it runs `clm approve`
- THEN `clm approve` refuses and says to run it in the human's own terminal

**Untested**
That an adapter holds no rule of its own is a judgement in design and review; that an agent harness's declared abilities match what it really offers rests on its documentation, which CLM doesn't test. The generic tier is accepted with the two gaps above, since it needs no adapter at all; a project that needs every rule held by a program works from a recognised agent harness.
