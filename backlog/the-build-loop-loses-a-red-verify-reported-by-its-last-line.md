---
title: "The build loop loses a red verify reported by its last line"
kind: bug
components: [orchestrator.loop, plugin]
origin: 20261118-the-build-skill-gives
---

Step: the build loop prints `run verify clm verify --due ...`; the build skill and `orchestrator.loop/plan-verify` say to report the run's last line with `clm build returned verify --result <line>`, and the session runs the command as a background Bash command, whose output holds stdout and stderr together.

What happens: when the verify fails, `clm verify` prints `verify failed <sha> culprit ... components ...` on stdout and then its notes on stderr, the last being `clm verify: output is at <log>`. The run's last line is that note, so `clm build returned verify` gets a line that doesn't start `verify failed`, and the loop records no red result and puts nothing in the human's queue. The same holds for a passed verify whose mutation pass prints notes after the outcome line, harmlessly, since anything that isn't `verify failed` reads as green.

Expected: the loop learns a verify's outcome whatever order its streams' lines come in: the session reports the command's outcome line (the line starting `verify `) rather than its last line, or `clm build returned` reads the outcome another way (the ledger's `verified` record). `plan-verify` states "last line", so the fix needs a spec change.
