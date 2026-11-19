---
title: "The build skill reports a verify run's last line, not its outcome line"
kind: deferred
components: [plugin]
origin: 20261118-the-build-loop-loses
---

`plugin/skills/build/SKILL.md` tells the session, for a `run` action, to report the command's last line with `clm build returned`, and, in the paragraph after the actions, to report every background command's return with its last line. A failed `clm verify --due` prints its `verify failed ...` line on stdout and then notes on stderr, the last `clm verify: output is at <log>`, so the last line of the background command's output is a note, and the loop records no red result. `orchestrator.loop/plan-verify` now says the session reports a verify run's outcome line, the last line of its output that starts `verify `, or `none` when no line does.

This is due as soon as `20261118-the-build-loop-loses` is on `main`: the sentence is right with the loop as it is built now, which records any line starting `verify failed`, and with the refusal `plan-verify` adds. Change the `run` bullet to say: for `verify`, report the last line of the output that starts `verify `, or `none` when no line does; for any other run, its last line; with no `--cause`, however the command ended. Make the paragraph's "the last line as it stands" say the same for `verify`. Resolve this item in that commit with the trailer `Resolves: backlog/the-build-skill-reports-a-verify-run-s-last-line-not-its-outcome`.
