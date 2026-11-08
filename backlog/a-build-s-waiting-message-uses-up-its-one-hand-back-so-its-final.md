---
title: "A build's waiting message uses up its one hand-back, so its final report is refused"
kind: bug
components: [orchestrator.run, plugin]
---

Step: /clm:build, a background build subagent launched from the `clm run` prompt.

What happened: the build ended a turn while its own reviewers still ran, as the prompt's `background-work-stopped` rule says, with a message ending `waiting: <command>`. Claude Code delivered that message as the subagent's hand-back report. When the build later finished and landed, its final report (ending `landed <sha>`) was refused by Claude Code with "one report per result", so the orchestrating session never received the closing line. The orchestrating session only got it by resuming the subagent with SendMessage and asking for it, a step the build skill doesn't describe.

Expected: the build's final report, with its closing line, reaches the orchestrating session without extra steps.

Rules involved: `orchestrator.run/background-work-stopped` (the waiting message is "not your final message") and the build skill's `waiting:` handling (`plugin/build-...`, `unclosed-build-reads-finish`). Under the current hand-back mechanism the waiting message is delivered as the final report. The prompt could tell the build not to send a report while waiting (end its turn without one) or the build skill could say to ask a build for its report with SendMessage when the hand-back was refused. Either is a spec change.
