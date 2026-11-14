---
title: "clm run writes the build prompt to a file and prints a short launcher"
date: 2026-10-26
status: accepted
---

`clm run <node>` writes the build's prompt, whose `clm context` bundle runs to tens of KB, to `.clm/prompts/<node>.md` in the checkout it runs in, and prints on stdout only a launcher of a few lines naming that file by its absolute path. `implement` still passes `clm run`'s stdout unchanged as the Agent tool's prompt; the subagent reads the file with the Read tool and follows it.

**Why.** To pass a 27-57 KB prompt unchanged, the orchestrating session read it into its own context and emitted it again as a tool argument: hundreds of KB per wave spent copying text, and in one session emitting it got the response stopped by a safety classifier, so the build couldn't launch as the skill described. Reading the file in the subagent worked when an orchestrator improvised it. The subagent's context holds the prompt either way; the orchestrator's no longer does.

**Why stdout stays the launcher.** `implement`'s instruction, pass stdout unchanged, stays true, so the skill and the CLI don't have to change together, and no expand-then-contract step is needed (`expand-contract`).

**Where.** `.clm/` is already outside git through `.git/info/exclude`, and the orchestrator's checkout outlives every build worktree, so the file is there when the subagent starts and until the next `clm run` of the node replaces it. A path in the common git directory was rejected: Claude Code treats `.git/` as protected, and a build has no reason to look there.

**Alternatives.** A `--prompt-file` flag alongside the old stdout keeps two outputs for one caller (`no-dead-code`). Having the subagent run `clm run` itself would put the claim and the leftover cleanup inside a worktree the build may be standing in.

**Drafts and folds.** `clm draft` and `clm draft --fold` write their prompts the same way, to `.clm/prompts/draft/<id>.md` and `.clm/prompts/fold/<first id>.md`, and print a launcher. A draft prompt holds the items rather than a context bundle, so it is a few KB, but passed inline the triage session would still read and re-emit one per group, and the launcher keeps every subagent prompt CLM writes on one pattern.
