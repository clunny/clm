---
title: "Antigravity CLI as a supported agent harness: adapter, plugin package and hooks"
kind: idea
components: [agent-harness, plugin, plugin.hooks, plugin.hooks.guard]
---

Stage 2 of `20261112-any-harness` (`any-agent-harness`). Add `Antigravity` to `clm-agent-harness`: its environment marker, its abilities, its hook events and decisions, and a package in `plugin/` (`plugin.json` at the plugin root and `hooks.json`, as the Antigravity CLI's plugin docs give them), with skills read from the same `plugin/skills/`. Means the Antigravity CLI (`agy`) and its SDK, not the IDE. First settle whether its pre-tool hook can hold a command for the human's yes (`confirm`); until shown, `confirm` is false and approval goes to the human's terminal.
