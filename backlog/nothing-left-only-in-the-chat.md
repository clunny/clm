---
title: "Wrap-up makes sure nothing important is left only in the chat"
kind: idea
components: [plugin, orchestrator.checkpoint]
---

## Intent

`clm checkpoint` (to be `clm wrap-up`, `plain-terms-renames`) makes sure a session's work is safe on `origin` before it says the session can end, but it reads only git. What exists only in the conversation is lost when the session ends or is cleared: a choice made in discussion, a fact the human gave, a loose end nobody wrote down. Today each skill's earlier steps catch some of it (decisions to `docs/decisions/`, deferred work to `backlog/`, `## Open questions`, `clm defect`), but no final step asks "is anything that matters still only in this conversation?", so whether it is caught depends on each skill remembering.

The idea: ending a session (`wrap-up-session`) also has the session review the conversation and put anything that matters into the repo, as a decision, a backlog item (`question`, `idea`, `deferred`), an edit to a live change or a defect report, before it says the session can end. Claude's own memory isn't a place for it (`CLAUDE.md`: memory holds only the human's preferences and facts about the machine).

## Settled

- The review lives in the skills, since only the session can read its own conversation; the command keeps checking git (`deterministic-enforcement`: the review advises, the git check gates).

## Open questions

- One shared final step every skill (`propose`, `implement`, `triage`, `init`) runs before `clm checkpoint`, or a skill of its own (a `/clm:wrap-up`) the human can also run at any time?
- Should `CAPABILITIES.md`'s `wrap-up-session` say it (needs review), with a scenario such as "a decision made only in chat is in `docs/decisions/` before the session says it can end"?
- Can any of it be checked by a program, or is it `#### Untested` judgement?
- Where does what it finds go when the session is unattended (a `clm run` build or a draft): the page body, or backlog items?
