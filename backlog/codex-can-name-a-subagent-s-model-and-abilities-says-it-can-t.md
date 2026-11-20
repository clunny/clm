---
title: "Codex can name a subagent's model, and abilities says it can't"
kind: idea
components: [agent-harness]
origin: 20261119-wave-1-by-the-rule
---

`agent-harness/abilities` gives Codex `model_choice` false. Codex's source, read on 2026-11-19 at `d0264b75`, shows its `spawn_agent` tool takes `model` and `agent_type`, and its agents load from `.codex/agents/` and `~/.codex/agents/` with a `model` of their own, though not from a plugin's manifest, so `named_agents` stays false. With `model_choice` true and a `build_model` for Codex, builds under Codex would launch on a named model as they do under Claude Code (`model-per-launch`).

It changes behaviour, so it is a change of its own and not part of the pass that found it: which model, and whether Codex's `subagents` work under the build loop as Claude Code's do, are unmeasured. Two smaller findings from the same reading: Codex's `apply_patch` tool also writes `tool_input.command`, so a hook must look at `tool_name` (the guard does); and a block whose reason is empty doesn't block under Codex, which treats exit 2 with nothing on stderr as a failed hook and runs the command.
