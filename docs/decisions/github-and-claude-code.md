---
title: "GitHub and Claude Code are decided platform dependencies"
date: 2026-10-22
status: superseded
superseded_by: host-adapters
---

CLM runs on GitHub (PRs, labels, Actions, `gh`, the `web` GitHub App) and Claude Code (the plugin, hooks, skills, `claude -p`). Code may use either directly, with no abstraction layer for other forges or agents. Moving off either is a spec change. `forge-adapters` in the backlog records what supporting Azure DevOps or plain git would take.

Cheap portability is still preferred where it costs nothing: logic lives in the `clm` CLI and hooks only call it (`hook-logic-in-clm`), and state lives in git (`git-is-truth`), so forge-specific code stays at the edges.

This is a decision, not a Layer 1 invariant. Invariants state properties that every change must keep, checked by programs. A platform choice is a scope decision a later change may revisit, and `ARCHITECTURE.md` names it in a short `## Platform` section so every context bundle carries it.
