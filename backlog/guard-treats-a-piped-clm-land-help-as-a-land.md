---
title: "Guard treats a piped clm land --help as a land"
kind: bug
components: [plugin.hooks.guard]
---

Step: reading the help for `clm land` through a pipe, as `clm land --help | head`.

What happened: the PreToolUse guard refused the command under `guard-refuses-piped-land`.

Expected: the refusal applies only to a command that lands. `--help` lands nothing and has no verdict line for a pipe to hide, so it should pass.

Rule involved: `guard-refuses-piped-land`.
