---
title: "The CLM panel opens itself when /clm:build starts, from its own hook"
date: 2026-11-09
status: accepted
---

Builds run for a long time in background subagents, and the panel's Flight tab is where they show. `/clm:build` should open it. The panel's hooks module does it: it hooks Claude Code's `skill.prompt` event for the `clm:build` skill, passes the prompt through unchanged, and opens the pane on Flight when it isn't already open (`plugin.panel/build-opens-panel`).

- **Only when installed, by construction.** The rule lives in `clm-panel`, so without the panel there is no hook and `/clm:build` behaves as before; the `clm` plugin never needs to know whether the panel is there.
- **Code, not prose.** The opening is a tested function of the panel, not a line in `skills/build/SKILL.md` a model may skip (`deterministic-enforcement`).
- **Both ways in.** `skill.prompt` fires for a typed `/clm:build` and for the model's Skill tool call alike; `command.run` would see only the typed one.
- **The person's layout wins.** An open pane keeps its tab; the hook opens nothing then, and opens again on a later `/clm:build` only once the person has closed it.

Rejected:
- **The build skill asks the model to run `/clm:view`.** A model can't type a slash command; through the Skill tool the panel's `command.run` hook may not see it, and without the panel the command file would run `clm view queue` in the chat instead.
- **Opening on the first build subagent's spawn.** It would reopen a pane the person closed mid-build, and it isn't tied to the person's ask.
