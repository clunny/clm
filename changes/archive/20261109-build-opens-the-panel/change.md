---
status: done
---

## Intent

Starting `/clm:build` opens the CLM panel on its Flight tab when `clm-panel` is installed and the pane isn't already open, so the person watches the builds land without typing `/clm:view`. The panel does it itself, from the `skill.prompt` event Claude Code raises when the `clm:build` skill's prompt is expanded; the build skill's text doesn't change.

## Problem

`/clm:build` runs builds in background subagents for a long time, and the Flight tab is where they show: each build's row under its component, its steps, tokens and cost (`build-rows`). Today the pane opens only when the person types `/clm:view`, which most forget until they wonder what the builds are doing.

## Approaches

- **The panel opens itself when the build skill starts (chosen).** `clm-panel`'s hooks module hooks `skill.prompt` for the `clm:build` skill, leaves its prompt as computed, and opens the pane as `/clm:view` with no argument does when it isn't open. `skill.prompt` fires whether the person typed `/clm:build` or the model called the Skill tool. "When it's installed" holds by construction: without `clm-panel` the hook doesn't exist and nothing changes. It is code, tested with the panel's other rules, not prose a model may skip (`deterministic-enforcement`). Recorded as `build-opens-panel-from-the-panel`.
- **The build skill tells the model to open it.** A model can't type a slash command; it could call the Skill tool with `clm:view`, but whether the panel's `command.run` hook sees a Skill-tool call is unsure, and without the panel the command file would run `clm view flight` in the chat, which the person didn't ask for. Rejected.
- **Open on the first build subagent's spawn.** Covers builds started any other way, but it isn't what was asked, and reopens a pane the person closed mid-build. Rejected.

## Outcome

`plugin.panel` gains `build-opens-panel`, and its Contract says the pane also opens when `/clm:build` starts. The guide's "See where things stand" says so.

## Constraints

- The skill's prompt passes through unchanged, and a failure to open never stops the build.
- An open pane is left as it is: its tab, its capability page.
- Outside a CLM repository, nothing opens, as with `/clm:view`.
