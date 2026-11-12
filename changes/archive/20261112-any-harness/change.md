---
status: done
---

## Intent

Let a project under CLM be worked from Claude Code, OpenAI Codex, or any other coding agent that reads `AGENTS.md` and runs shell commands, with contributors on different tools, or one person switching tools mid-change, sharing the same rules and the same state. CLM's rules already live in the `clm` binary and in git rather than in the agent (`deterministic-enforcement`, `git-is-truth`, `hook-logic-in-clm`); this change makes the agent a supported, swappable adapter: the `platform` constraint names agent harnesses instead of Claude Code alone, a new `agent-harness` component recognises the agent harness a command runs under and states what it can do, the hooks and the guard answer in each agent harness's own format, `clm approve` refuses under an agent harness that can't make the human confirm it, the plugin ships a Codex package beside its Claude Code one with the same skills and agents, the skills follow what `clm agent-harness` says the agent harness can do, and `clm init` writes the project's instructions to `AGENTS.md`, with `CLAUDE.md` importing it.

This is stage 1 of three. Antigravity CLI support (stage 2) and per-harness runner launches and cost tracking (stage 3) are backlog items.

## Problem

`ARCHITECTURE.md`'s `platform` says CLM is built on git and Claude Code, and the plugin, its hooks, the guard's `ask` decision that makes an approval the human's (`approve-in-session`), the files `clm init` writes (`CLAUDE.md`, `.claude/settings.json`) and the skills' wording (the Agent tool, `isolation: "worktree"`, `model: "sonnet"`) all assume it. A Codex user can run the `clm` binary, but gets no skills, no guard, no session-start state and no instructions: Codex reads `AGENTS.md`, not `CLAUDE.md`. Worse, nothing in `clm approve` itself checks that a human confirmed it: under an agent with no guard, an agent could approve its own change.

Codex (CLI 0.117 and later) and the Antigravity CLI now offer the same kinds of extension Claude Code does: plugins with a manifest, `SKILL.md` skills (the same open format), lifecycle hooks with a pre-tool hook, `AGENTS.md`, subagents and a headless mode. None of them lets a plugin draw a pane, so the CLM panel stays Claude Code's.

## Approaches

**How the plugin reaches each agent harness.**
1. **One neutral source, packaged per agent harness** (chosen). The skills and agents are written once in neutral words; `plugin/` carries a manifest per agent harness (`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`) and each agent harness's hook wiring, every hook calling `clm hook <event> --agent-harness <h>`, so only the event's input and output format differs per agent harness, and it lives in `clm`. Skills can't drift between agent harnesses, and an agent harness is an adapter, not a fork.
2. A hand-kept plugin copy per agent harness. Quick to start; three copies of every skill, which drift, breaking a project worked from several tools. Rejected.
3. `AGENTS.md` and skills only, no hooks, for every agent harness. Simplest, but every agent harness loses the push guard and in-chat approval. Rejected.

**How an approval stays the human's** (`approval-policy`).
- **A. Approve only where a guard can ask** (chosen). `clm approve` refuses when it runs under a recognised agent harness that can't make the human confirm a command; it runs there only from the human's own terminal. Where the agent harness can ask, in-chat approval stays as today.
- B. Approve only in the human's own terminal, on every agent harness. Simplest and strongest, but takes away the in-chat yes Claude Code users have. Rejected by the human.
- C. Signed approvals with a key that needs a touch or passphrase per use. Strong, but heavy to set up for every contributor; signed repositories already have `signed-approve` on top of either A or B.

**Instructions for mixed teams.** `AGENTS.md` holds the project's instructions and `CLAUDE.md` holds `@AGENTS.md`, which Claude Code reads as an import, so every agent harness reads one text (chosen). Keeping two files with the same rules would drift.

**The panel.** Neither Codex CLI nor the Antigravity CLI offers a plugin a way to draw a pane (checked 2026-11-12; secondary sources), so `watch-beside-chat` stays Claude Code's; on other agent harnesses `/clm:view`'s text answer and `clm view` serve. A backlog idea records a pane on other agent harnesses.

**The worktree directory.** `.claude/worktrees/` stays CLM's worktree directory on every agent harness: it is only a directory, and renaming it would touch every tidy-up for nothing.

## Outcome

- `ARCHITECTURE.md`: `platform` names git and the supported agent harnesses; a new `agent-harness-adapters` states that an adapter only recognises its agent harness, translates its events and declares its abilities; `approval-policy` and `guard-blocks-push-to-main` say "the agent harness's confirmation" and "each agent harness's guard".
- `CAPABILITIES.md`: a new `work-from-any-agent-harness`; `install-and-release` covers the Codex package; `watch-beside-chat` says it is Claude Code's.
- `agent-harness` (new, crate `clm-agent-harness`): `detect`, `abilities`, `hook-events`.
- `clm` (root): `agent-harness-command` wires `clm agent-harness`.
- `plugin.hooks.guard`: `guard-per-agent-harness`. `plugin.hooks`: `hooks-per-agent-harness`.
- `land.approval`: `approve-refuses-unconfirming-agent-harness`.
- `plugin`: Contract names both packages; `codex-package`, `codex-hooks-wired`, `skills-follow-agent-harness` (shared, with `#### Needs` on each child's).
- `plugin.propose`, `plugin.build`, `plugin.triage`, `plugin.init`: each skill's requirement to follow `clm agent-harness`.
- `init`: `AGENTS.md` written with the instructions and `CLAUDE.md` importing it; the checks on `CLAUDE.md`'s text read `AGENTS.md`.
- `upgrade`: `migration-agents-md` moves an existing project's `CLAUDE.md` into `AGENTS.md`.
- `gate`, `check`, `sync`, `ledger`, `web.worker.platform`, `check.health`: `AGENTS.md` is a spec path wherever `CLAUDE.md` is.

## Constraints

- `approval-policy`, `humans-review-intent`, `guard-blocks-push-to-main`, `hook-logic-in-clm`, `method-in-the-plugin`, `expand-contract` (`clm hook guard` without `--agent-harness` keeps meaning Claude Code), `yagni` (no adapter for an agent harness nobody has asked for; the generic tier needs none).
- An agent harness CLM doesn't recognise looks like the human's terminal to `clm approve`. For it, what keeps an approval the human's is the instruction in `AGENTS.md`, signed approvals where the repository signs, and `clm ledger verify` after the fact: no weaker than a Claude Code session with the plugin turned off today. `approval-policy`'s `#### Untested` says so.

## Deferred

`backlog/antigravity-cli-agent-harness` (stage 2), `backlog/runner-per-agent-harness` and `backlog/effort-from-other-agent-harnesses` (stage 3), `backlog/neutral-build-and-draft-prompts` and `backlog/pane-on-other-agent-harnesses`, and `backlog/contributing-md-lists-agents-md` (deferred, for this repository's `CONTRIBUTING.md`). The decision is `any-agent-harness`.

## Open questions

- Codex's facts come from its documentation, read at build time: the environment marker it sets in every command, its hook event and decision JSON, the variable a plugin's hook command uses for the plugin's own directory, and the file and format of a repository's plugin marketplace (`.agents/plugins/` is assumed in `plugin`'s `paths`). The builds of `agent-harness`, `plugin.hooks` and `plugin` read them, and correct `plugin`'s `paths` through a spec change if the marketplace lives elsewhere.
- **The generic tier keeps two gaps, accepted in writing** (architecture audit). An agent CLM doesn't recognise looks like the human's terminal: `clm approve` can't refuse under it, and no guard blocks its direct push to `main`. `agent-harness-adapters`, `approval-policy` and `guard-blocks-push-to-main` now say so under `#### Untested`, naming what holds instead (the `AGENTS.md` instruction, signed approvals, branch protection, `clm ledger verify`). A TTY confirmation for `clm approve` was considered and left out, since an agent's shell can be given a terminal. The alternative is to drop the generic tier from `platform` and support only recognised agent harnesses.
- **The word**: settled by the human as "agent harness", used whole in every ID, the command, the flag and prose, so it never collides with the glossary's test harness.
- Whether Codex's pre-tool hook can hold a command for the human's yes. Until a build shows it can, `codex`'s `confirm` is false and approvals there go to the human's terminal.
