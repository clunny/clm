---
status: applied
---

## Intent

Let a project under CLM be worked from Claude Code, OpenAI Codex, or any other coding agent that reads `AGENTS.md` and runs shell commands, with contributors on different tools, or one person switching tools mid-change, sharing the same rules and the same state. CLM's rules already live in the `clm` binary and in git rather than in the agent (`deterministic-enforcement`, `git-is-truth`, `hook-logic-in-clm`); this change makes the agent a supported, swappable adapter: the `platform` constraint names harnesses instead of Claude Code alone, a new `harness` component recognises the harness a command runs under and states what it can do, the hooks and the guard answer in each harness's own format, `clm approve` refuses under a harness that can't make the human confirm it, the plugin ships a Codex package beside its Claude Code one with the same skills and agents, the skills follow what `clm harness` says the harness can do, and `clm init` writes the project's instructions to `AGENTS.md`, with `CLAUDE.md` importing it.

This is stage 1 of three. Antigravity CLI support (stage 2) and per-harness runner launches and cost tracking (stage 3) are backlog items.

## Problem

`ARCHITECTURE.md`'s `platform` says CLM is built on git and Claude Code, and the plugin, its hooks, the guard's `ask` decision that makes an approval the human's (`approve-in-session`), the files `clm init` writes (`CLAUDE.md`, `.claude/settings.json`) and the skills' wording (the Agent tool, `isolation: "worktree"`, `model: "sonnet"`) all assume it. A Codex user can run the `clm` binary, but gets no skills, no guard, no session-start state and no instructions: Codex reads `AGENTS.md`, not `CLAUDE.md`. Worse, nothing in `clm approve` itself checks that a human confirmed it: under an agent with no guard, an agent could approve its own change.

Codex (CLI 0.117 and later) and the Antigravity CLI now offer the same kinds of extension Claude Code does: plugins with a manifest, `SKILL.md` skills (the same open format), lifecycle hooks with a pre-tool hook, `AGENTS.md`, subagents and a headless mode. None of them lets a plugin draw a pane, so the CLM panel stays Claude Code's.

## Approaches

**How the plugin reaches each harness.**
1. **One neutral source, packaged per harness** (chosen). The skills and agents are written once in harness-neutral words; `plugin/` carries a manifest per harness (`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`) and each harness's hook wiring, every hook calling `clm hook <event> --harness <h>`, so only the event's input and output format differs per harness, and it lives in `clm`. Skills can't drift between harnesses, and a harness is an adapter, not a fork.
2. A hand-kept plugin copy per harness. Quick to start; three copies of every skill, which drift, breaking a project worked from several tools. Rejected.
3. `AGENTS.md` and skills only, no hooks, for every harness. Simplest, but every harness loses the push guard and in-chat approval. Rejected.

**How an approval stays the human's** (`approval-policy`).
- **A. Approve only where a guard can ask** (chosen). `clm approve` refuses when it runs under a recognised harness that can't make the human confirm a command; it runs there only from the human's own terminal. Where the harness can ask, in-chat approval stays as today.
- B. Approve only in the human's own terminal, on every harness. Simplest and strongest, but takes away the in-chat yes Claude Code users have. Rejected by the human.
- C. Signed approvals with a key that needs a touch or passphrase per use. Strong, but heavy to set up for every contributor; signed repositories already have `signed-approve` on top of either A or B.

**Instructions for mixed teams.** `AGENTS.md` holds the project's instructions and `CLAUDE.md` holds `@AGENTS.md`, which Claude Code reads as an import, so every harness reads one text (chosen). Keeping two files with the same rules would drift.

**The panel.** Neither Codex CLI nor the Antigravity CLI offers a plugin a way to draw a pane (checked 2026-11-12; secondary sources), so `watch-beside-chat` stays Claude Code's; on other harnesses `/clm:view`'s text answer and `clm view` serve. A backlog idea records a pane on other harnesses.

**The worktree directory.** `.claude/worktrees/` stays CLM's worktree directory on every harness: it is only a directory, and renaming it would touch every tidy-up for nothing.

## Outcome

- `ARCHITECTURE.md`: `platform` names git and the supported harnesses; a new `harness-adapters` states that an adapter only recognises its harness, translates its events and declares its abilities; `approval-policy` and `guard-blocks-push-to-main` say "the harness's confirmation" and "each harness's guard".
- `CAPABILITIES.md`: a new `work-from-any-harness`; `install-and-release` covers the Codex package; `watch-beside-chat` says it is Claude Code's.
- `harness` (new, crate `clm-harness`): `detect`, `abilities`, `hook-events`.
- `clm` (root): `harness-command` wires `clm harness`.
- `plugin.hooks.guard`: `guard-per-harness`. `plugin.hooks`: `hooks-per-harness`.
- `land.approval`: `approve-refuses-unconfirming-harness`.
- `plugin`: Contract names both packages; `codex-package`, `codex-hooks-wired`, `skills-follow-harness` (shared, with `#### Needs` on each child's).
- `plugin.propose`, `plugin.build`, `plugin.triage`, `plugin.init`: each skill's requirement to follow `clm harness`.
- `init`: `AGENTS.md` written with the instructions and `CLAUDE.md` importing it; the checks on `CLAUDE.md`'s text read `AGENTS.md`.
- `upgrade`: `migration-agents-md` moves an existing project's `CLAUDE.md` into `AGENTS.md`.
- `gate`, `check`, `sync`, `ledger`, `web.worker.platform`, `check.health`: `AGENTS.md` is a spec path wherever `CLAUDE.md` is.

## Constraints

- `approval-policy`, `humans-review-intent`, `guard-blocks-push-to-main`, `hook-logic-in-clm`, `method-in-the-plugin`, `expand-contract` (`clm hook guard` without `--harness` keeps meaning Claude Code), `yagni` (no adapter for a harness nobody has asked for; the generic tier needs none).
- A harness CLM doesn't recognise looks like the human's terminal to `clm approve`. For it, what keeps an approval the human's is the instruction in `AGENTS.md`, signed approvals where the repository signs, and `clm ledger verify` after the fact: no weaker than a Claude Code session with the plugin turned off today. `approval-policy`'s `#### Untested` says so.

## Deferred

`backlog/antigravity-cli-harness` (stage 2), `backlog/runner-per-harness` and `backlog/effort-from-other-harnesses` (stage 3), `backlog/neutral-build-and-draft-prompts` and `backlog/pane-on-other-harnesses`, and `backlog/contributing-md-lists-agents-md` (deferred, for this repository's `CONTRIBUTING.md`). The decision is `any-harness`.

## Open questions

- Codex's facts come from its documentation, read at build time: the environment marker it sets in every command, its hook event and decision JSON, the variable a plugin's hook command uses for the plugin's own directory, and the file and format of a repository's plugin marketplace (`.agents/plugins/` is assumed in `plugin`'s `paths`). The builds of `harness`, `plugin.hooks` and `plugin` read them, and correct `plugin`'s `paths` through a spec change if the marketplace lives elsewhere.
- **The generic tier keeps two gaps, accepted in writing** (architecture audit). An agent CLM doesn't recognise looks like the human's terminal: `clm approve` can't refuse under it, and no guard blocks its direct push to `main`. `harness-adapters`, `approval-policy` and `guard-blocks-push-to-main` now say so under `#### Untested`, naming what holds instead (the `AGENTS.md` instruction, signed approvals, branch protection, `clm ledger verify`). A TTY confirmation for `clm approve` was considered and left out, since an agent's shell can be given a terminal. The alternative is to drop the generic tier from `platform` and support only recognised harnesses.
- **The word "harness"** (architecture audit, `plain-terms`). The glossary already used "Harness" for the capability suite's test harness. Kept, with "Agent harness" and "generic tier" defined beside it and each entry saying it isn't the other; the alternative is another name, such as "agent tool", for the component and the command.
- Whether Codex's pre-tool hook can hold a command for the human's yes. Until a build shows it can, `codex`'s `confirm` is false and approvals there go to the human's terminal.
