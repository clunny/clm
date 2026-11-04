---
title: "The defect ask quotes the report, and refuses a report it can't show"
date: 2026-11-04
status: accepted
---

The guard's `clm defect` ask (`defect-reports`) tells the human to confirm only after reading the report, so the prompt that asks must hold the report. Claude Code shows a `PreToolUse` hook's `permissionDecisionReason` in its prompt; it doesn't promise to show the command beside it, and from an outside project the first prompt showed the reason alone, with the command only in a second, ordinary permission prompt. So the guard quotes, in its reason, each `clm defect` segment's own line (its title and components) and the report it sends.

The guard quotes only text it knows the shell will send: the body of the heredoc that feeds the segment's standard input, when its delimiter is quoted or its body holds nothing the shell expands (`$`, `` ` ``, `\`). A report from anywhere else (a file, a pipe, a here-string, a variable, an expanding heredoc) is refused with exit 2, and the reason tells the session to resend it as a `<<'EOF'` heredoc; the session reads that reason and retries, so the cost is one extra command. The guard doesn't read a named file in its place: the file could change between the prompt and the run, so what the human read might not be what is filed, and a yes to a report no one could see isn't informed consent.

Rejected: rewording the question to point at the next prompt (it depends on Claude Code's undocumented dialog layout, which may differ by permission mode, and the first prompt still shows nothing); having `clm defect` print the report and ask (under Claude Code it has no terminal to ask on, and the guard's prompt is what makes the yes the human's); asking with a warning when the report can't be shown (the human chose refusing).
