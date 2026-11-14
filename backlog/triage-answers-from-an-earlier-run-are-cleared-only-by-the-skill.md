---
title: "Answers from an earlier triage run are cleared only if the skill remembers to"
kind: idea
components: [orchestrator.triage]
---

The triage skill records the human's answers in `.clm/answers.md` and passes the file to every `clm draft --answers`. It is told to remove the file before `clm triage`, so a previous run's answers don't reach this run's drafts. The requirement that pinned that sentence, `plugin.triage/triage-answers`, was a scan of prompt text and is deleted in `20261113-phase-2-delete`.

`clm triage` starts every run, so it could clear the file itself.
