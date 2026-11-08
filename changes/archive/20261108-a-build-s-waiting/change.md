---
status: done
resolves: [a-build-s-waiting-message-uses-up-its-one-hand-back-so-its-final]
---

## Intent

A build or draft hands back one report, its final message. The `clm run` and `clm draft` prompts tell the session that a hand-back tool accepting one report per run, such as Claude Code's `SubagentHandback`, is never called for the `waiting: <command>` message that ends a turn while its own `clm land` runs: that turn ends as plain text, and the tool is called once, with the final message ending `landed <sha>`, `proposed <id> review` or `stopped: <reason>`. The closing line then reaches the orchestrating session with no extra step. Decision: `one-report-per-run`. Resolves `backlog/a-build-s-waiting-message-uses-up-its-one-hand-back-so-its-final`.

## Problem

`background-work-stopped` has a build end a waiting turn with a `waiting: <command>` message, "not your final message", assuming each turn's last message reaches the orchestrator and a later one can follow. Claude Code now delivers a subagent's result through a hand-back tool that takes one report per run. A build sent its waiting message through it, and its final report was refused ("one report per result"), so `/clm:build` never got the closing line; it only got it by resuming the subagent with `SendMessage`, a step the skill doesn't describe. Drafts follow the same rule in `clm draft`'s prompt (`orchestrator.triage/background-work-stopped`) and fail the same way, so the change fixes both prompts.

## Approaches

- **Prompt: no hand-back while waiting** (chosen). The prompt is where the waiting message is defined, and the fix keeps the report from being lost. The skills need no change: their waiting rules (`plugin/build-waiting-builds`, `plugin/triage-waiting-drafts`) already treat a subagent that Claude Code marks as still running background work as not returned, which is what a turn ended without a hand-back leaves.
- **Skill: ask the build for its report with `SendMessage`.** Rejected: the orchestrator sees only a build that ended without a closing line, which `unclosed-build-reads-finish` already judges; it would repair a loss the prompt can prevent, at a resume's cost each time.
- **Drop the `waiting:` line.** Rejected: under a harness whose turn-end text is the result, the orchestrator would read the build as returned and finish it while its land runs.

## Outcome

Each prompt's closing paragraph says it hands back one report: never through the hand-back tool for a waiting turn, once for the final message. `plugin` changes nothing, though the item names it.

## Edge cases

- A build or draft that sends a waiting report anyway still loses its closing line; for a build, `unclosed-build-reads-finish` judges from `clm run --finish` whether it landed, and a draft is handled as any draft that ends without a closing line is now.
