---
title: "The prompts clm run and clm draft write name Claude Code's tools"
kind: idea
components: [orchestrator.run, orchestrator.triage]
---

Found designing `20261112-any-harness` (`any-harness`). The skills follow `clm harness`, but the build prompt `clm run` writes and the draft prompts `clm draft` writes still name the Agent tool, `model: "sonnet"`, `EnterWorktree` and `CLAUDE.md`. A build or draft launched from Codex reads them as Claude Code's words. Write them in harness-neutral words, reading `clm harness` where a step depends on what the harness can do, and naming the project's instructions as `AGENTS.md`.
