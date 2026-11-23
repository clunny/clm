---
title: "The build skill has no sentence for the human's notes"
kind: deferred
components: [plugin]
origin: 20261122-wave-4-loop-by-the-rule
---

`plugin/skills/build/SKILL.md` tells the session to start the loop with `clm build start --session-model <alias>` and to give each subagent the launch file's text unchanged, so notes the human gives for every run of the session have no route but an edit to what the loop prints.

Once `orchestrator.loop/notes-reach-every-launch` is current, add to the paragraph that starts the loop a sentence telling the session, when the human gives notes for the runs of the loop (a file, or text it writes to a file of its own under `.clm/`), to pass that file with `--notes <file>` to `clm build start`, and never to add them to a launch's prompt itself. Resolve this item in that commit with the trailer `Resolves: backlog/the-build-skill-has-no-sentence-for-the-human-s-notes`.
