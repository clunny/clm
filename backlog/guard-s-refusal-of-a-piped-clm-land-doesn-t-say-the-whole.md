---
title: "Guard's refusal of a piped clm land doesn't say the whole command line was refused"
kind: bug
components: [plugin.hooks.guard]
---

The guard refuses a `clm land` whose output is piped (`guard-refuses-piped-land`). The refusal cancels the whole Bash command, including anything chained before the land: `git commit ... && clm land <branch> | tail` lost the commit. The message speaks only of the land, so the session believed the commit had run.

Expected: the refusal says the whole command line was refused and nothing in it ran.
