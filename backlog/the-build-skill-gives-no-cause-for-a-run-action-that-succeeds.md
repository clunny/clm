---
title: "The build skill gives no --cause for a run action that succeeds"
kind: bug
components: [plugin]
---

Step: the build loop printed `run verify clm verify --due --event work-done`. The session ran it as a background command and reported its last line with `clm build returned verify --result "<last line>"`.

What happened: the verify passed (every test passed, and its last line was `clm verify: mutation: 1 killed, 0 unproven, 0 not judged`). The build skill says any return that isn't `landed <sha>`, `proposed <id> review <url>` or `waiting: <command>` needs a `--cause`. Every cause it lists (limit, disk, outage, tool, work, context, blocked, stopped) describes a failure. The session had to pick one for a run that succeeded, so it passed `work`, which the loop may count as a failed attempt.

Expected: the skill, or `clm build returned`, says how to report a `run` action's command that ended normally: with no `--cause`, a cause meaning it finished, or the loop reading the command's own outcome line. The same gap applies to a verify whose last line is its mutation count while an earlier step failed.
