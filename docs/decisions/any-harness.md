---
title: "Agent harnesses are adapters: one plugin source, fail-closed approval, AGENTS.md for instructions"
date: 2026-11-12
status: accepted
---

CLM is worked from Claude Code, Codex, and, as a generic tier, any agent that reads `AGENTS.md` and runs shell commands (`platform`, `harness-adapters`, `20261112-any-harness`). The human asked for parity of the method across harnesses, with mixed teams and tool switching mid-change, and chose Claude Code, Codex and the Antigravity CLI, in that order of stages.

**Parity of the method, tiers for conveniences.** Everything enforced already lives in `clm` and git (`deterministic-enforcement`, `git-is-truth`, `hook-logic-in-clm`), so specs, claims, approvals, gates and lands are the same on every harness. What a harness lacks degrades to a path that keeps the rule: approval in the human's own terminal where the harness can't ask, `clm land`'s gate where no guard runs, builds one at a time without subagents, the session's own model without model choice. The panel stays Claude Code's: neither the Codex CLI nor the Antigravity CLI lets a plugin draw a pane (checked 2026-11-12).

**One neutral plugin source, a package per harness.** Hand-kept plugin copies per harness would drift, and a project worked from several tools needs every tool to follow the same skill text. So `plugin/` carries a manifest and hook wiring per harness, the skills are written in harness-neutral words and read `clm harness` for what the harness can do, and each hook passes `--harness` to `clm`, where each harness's event and decision formats live (`clm-harness`).

**Approval fails closed.** Nothing in `clm approve` itself checked that a human confirmed it; Claude Code's guard prompt did. Under a recognised harness that can't make the human confirm a command, `clm approve` now refuses and points at the human's own terminal. An adapter's `confirm` stays false until the harness's documentation shows a hook that holds a command for a yes: a wrong false costs a trip to the terminal, a wrong true lets an agent approve. Approving only in the terminal everywhere was offered and declined, to keep Claude Code's in-chat yes; signed approvals with a per-use key were judged too heavy for every contributor. An agent CLM doesn't recognise looks like the terminal; there the instruction in `AGENTS.md`, signed approvals and `clm ledger verify` are what hold, no weaker than a Claude Code session with the plugin turned off.

**`AGENTS.md` holds the instructions.** Codex and Antigravity read `AGENTS.md`; Claude Code reads `CLAUDE.md`, which imports `AGENTS.md` with the line `@AGENTS.md`. One text, every harness. `AGENTS.md` is a spec path wherever `CLAUDE.md` is.

**`.claude/worktrees/` stays** the worktree directory on every harness: only a directory, and renaming it would touch every tidy-up for nothing.

**Staged.** Stage 1 is this change. The Antigravity CLI's adapter (stage 2), per-harness `clm runner` launches and cost tracking from other harnesses' transcripts (stage 3), and the build and draft prompts in harness-neutral words are backlog items.
