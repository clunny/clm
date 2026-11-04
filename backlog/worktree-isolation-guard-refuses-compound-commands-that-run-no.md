---
title: "Worktree-isolation guard refuses compound commands that run no VCS"
kind: bug
components: [plugin.hooks.guard]
origin: 20261104-web-app-links-and-early-access
---

In a session isolated under .claude/worktrees/, the guard refused several Bash commands that never invoke the version-control tool, saying they were too complex to verify that they stay inside the worktree: a brace group writing a file ({ printf ...; cat a b; } > out), an awk with a `|| { mkdir ...; awk ...; }` fallback, and a sed whose path argument came from a command substitution of `clm superpowers brainstorming`. It also refused a `clm defect` whose heredoc report merely mentioned the tool's name as a word. Expected: a command whose every segment runs a known tool that isn't the VCS (printf, cat, awk, sed, mkdir) passes, and words inside a quoted heredoc body fed to clm aren't read as a command. Instead each had to be split into separate calls, rewritten with the Write tool, or the report reworded. Rule involved: the guard's worktree-isolation check (worktree-safe-commands).
