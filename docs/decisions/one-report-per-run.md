---
title: "A build or draft hands back one report, its final message"
date: 2026-11-08
status: accepted
---

Under `builds-in-subagents`, a build or draft that ends a turn while its own background command (such as `clm land`) still runs ends that turn with a `waiting: <command>` message, which is not its final message (`orchestrator.run/background-work-stopped`, `orchestrator.triage/background-work-stopped`). The orchestrating session's skills treat such a result as not returned (`plugin/build-waiting-builds`, `plugin/triage-waiting-drafts`). That design assumed every turn's last message reaches the orchestrator and that a later one can follow it.

Claude Code now delivers a subagent's result through a hand-back tool (`SubagentHandback`) that accepts one report per run. A build sent its `waiting:` message through it; when its land finished, its final report, ending `landed <sha>`, was refused, and the orchestrating session never got the closing line, the pull request's findings or its `## CLM defects`, except by resuming the subagent with `SendMessage` and asking.

The prompts now say a build or draft hands back one report, its final message: a hand-back tool that accepts one report per run is never called for a waiting turn's message, which ends as plain text, and is called once, with the final message. A subagent that ends a turn without a hand-back while its background work runs stays running, which the skills' waiting rules already treat as not returned ("or that Claude Code marks as still running background work of its own"). The `waiting: <command>` line stays, since a harness that delivers each turn's last message still shows it, and the skills still read it.

Rejected:
- **Have the skills ask a build for its closing line with `SendMessage`** when its final hand-back was refused. The orchestrator can't see the refusal, only a build that ends without a closing line, which `unclosed-build-reads-finish` already judges from `clm run --finish`; asking repairs a report the prompt can keep from being lost in the first place, at a resume's cost each time (`redesign-before-workaround`).
- **Drop the `waiting:` line.** Under a harness whose turn-end text is the result, the orchestrator would read a waiting build's message as a return with no closing line and finish it while its land runs.

Accepted edge case: a build that sends a waiting report anyway still loses its closing line; `unclosed-build-reads-finish` tells the human whether it landed.
