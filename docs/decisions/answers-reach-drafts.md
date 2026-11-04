---
title: "The human's answers in a triage session reach drafts through the prompt, and each change quotes the ones it used"
date: 2026-10-30
status: accepted
---

`clm triage` lists `question` and `human` items as waiting on the human, and the `triage` skill passes `clm draft`'s launcher unchanged, so when the human answered questions in the triage session, set a policy for a set of items or gave every draft an instruction, nothing carried it to the drafts: the session wrote a scratch file and messaged each running subagent, racing its progress, and the answer lived only in the session.

So the answer travels in the prompt. The skill writes what the human says, verbatim, to `.clm/answers.md` in its checkout (outside git, removed at the start of each run), and passes `clm draft --answers .clm/answers.md` for every draft it launches after that. `clm draft` copies the file's text into the prompt under `## The human's answers`, a snapshot a later answer doesn't change. The draft designs from the answers that bear on its items, an answer outranking the item's own text, and quotes each one it used in `change.md` under `## The human's answers`, so the answer is reviewed with the change and archived with it. An answered `question` or `human` item is drafted as its own group, resolved by the change that settles it, with the fact it waited on written into the specs (`requirement-blocks`).

A draft already running never gets an answer: a message races its progress, and a prompt is read once. The summary names each draft launched before an answer that concerns it, for `/clm:propose`.

Rejected:
- **Editing the item's body with the answer first.** The draft reads `origin/main`'s backlog, so the edit would need its own landed branch before the draft starts; and editing an existing item makes a code branch mixed.
- **`--answer <slug> <text>` per item.** A policy for a set of items or an instruction for every draft answers no single item; one free-text file carries all three.
- **Messaging running drafts.** The side channel the bug reported: it races the draft and records nothing.
- **Asking the human before launching drafts.** Triage runs unattended; the human answers when they choose, and later drafts carry it.
