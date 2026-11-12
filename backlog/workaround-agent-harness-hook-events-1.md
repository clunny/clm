---
title: "Codex hook formats unwritten in spec"
kind: workaround
components: [agent-harness]
requirement: agent-harness/hook-events@cab71a98
---

The Design defers Codex's event and allow/deny formats to its documentation without writing them down. parse_event and render copy Claude Code's shapes (tool_input.command, stderr plus exit 2) for Codex, unverified. A spec change should state Codex's formats.
