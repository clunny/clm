---
title: "Unattended sessions run one command per call, and read clm sync's outcome from its output"
date: 2026-10-22
status: superseded
superseded_by: builds-in-subagents
---

`clm run`'s prompt tells sessions to run one command per Bash call, never joining commands with `&&`, `||`, `;` or `|`, and never starting one with `cd`. `clm sync` ends every run with `sync ok`, `sync incomplete` or `sync failed`, and the prompt goes by that line rather than the exit code.

**Why.** Under `dontAsk`, Claude Code denies a compound command unless each part matches an allow rule, and denies `cd` into another directory followed by `git`. Sessions were denied chained steps, and a `finish` session couldn't run `clm sync; echo $?` to see the exit code the prompt told it to act on. A session already starts in its worktree, so it never needs `cd`.

**Alternatives.** Widening the allow rules can't lift the `cd`-then-`git` denial, and it would loosen what unattended sessions may run (`unattended-commands`). A wrapper that runs a list of commands would be one more broad allow rule that runs anything.
