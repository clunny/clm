---
title: "Agent harnesses are adapters: one plugin source, fail-closed approval, AGENTS.md for instructions"
date: 2026-11-12
status: accepted
---

CLM is worked from Claude Code, Codex, and, as a generic tier, any agent that reads `AGENTS.md` and runs shell commands (`platform`, `agent-harness-adapters`, `20261112-any-harness`). The human asked for parity of the method across agent harnesses, with mixed teams and tool switching mid-change, and chose Claude Code, Codex and the Antigravity CLI, in that order of stages.

**Parity of the method, tiers for conveniences.** Everything enforced already lives in `clm` and git (`deterministic-enforcement`, `git-is-truth`, `hook-logic-in-clm`), so specs, claims, approvals, gates and lands are the same on every agent harness. What an agent harness lacks degrades to a path that keeps the rule: approval in the human's own terminal where the agent harness can't ask, `clm land`'s gate where no guard runs, builds one at a time without subagents, the session's own model without model choice. The panel stays Claude Code's: neither the Codex CLI nor the Antigravity CLI lets a plugin draw a pane (checked 2026-11-12).

**One neutral plugin source, a package per agent harness.** Hand-kept plugin copies per agent harness would drift, and a project worked from several tools needs every tool to follow the same skill text. So `plugin/` carries a manifest and hook wiring per agent harness, the skills are written in neutral words and read `clm agent-harness` for what the agent harness can do, and each hook passes `--agent-harness` to `clm`, where each agent harness's event and decision formats live (`clm-agent-harness`).

**Approval fails closed.** Nothing in `clm approve` itself checked that a human confirmed it; Claude Code's guard prompt did. Under a recognised agent harness that can't make the human confirm a command, `clm approve` now refuses and points at the human's own terminal. An adapter's `confirm` stays false until the agent harness's documentation shows a hook that holds a command for a yes: a wrong false costs a trip to the terminal, a wrong true lets an agent approve. Approving only in the terminal everywhere was offered and declined, to keep Claude Code's in-chat yes; signed approvals with a per-use key were judged too heavy for every contributor. An agent CLM doesn't recognise looks like the terminal, and runs no guard: under the generic tier `clm approve` can't refuse and nothing blocks a direct push to `main`. That is accepted and said openly in `agent-harness-adapters`, `approval-policy` and `guard-blocks-push-to-main`: what holds there is the instruction in `AGENTS.md`, signed approvals, the host's branch protection and `clm ledger verify` after the fact. A confirmation only a terminal gives (a TTY prompt) was considered and left out: an agent's shell can be given a terminal, so it would look like a program check without being one.

**`AGENTS.md` holds the instructions.** Codex and Antigravity read `AGENTS.md`; Claude Code reads `CLAUDE.md`, which imports `AGENTS.md` with the line `@AGENTS.md`. One text, every agent harness. `AGENTS.md` is a spec path wherever `CLAUDE.md` is.

**The word.** The human chose "agent harness", used whole everywhere: the component `agent-harness`, the crate `clm-agent-harness`, the command `clm agent-harness`, the flag `--agent-harness` and the constraint `agent-harness-adapters`. The glossary's existing "Harness" is the capability suite's test harness, so a bare "harness" never names an agent's tool.

**`.claude/worktrees/` stays** the worktree directory on every agent harness: only a directory, and renaming it would touch every tidy-up for nothing.

**Staged.** Stage 1 is this change. The Antigravity CLI's adapter (stage 2), per-harness `clm runner` launches and cost tracking from other agent harnesses' transcripts (stage 3), and the build and draft prompts in neutral words are backlog items.
