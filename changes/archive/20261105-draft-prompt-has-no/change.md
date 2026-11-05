---
status: done
resolves: [draft-prompt-has-no-route-for-an-answer-that-edits-a-human-task]
---

# A draft edits an item the human's answer keeps open

## Intent

The prompt `clm draft --answers` writes names a route for an answer that only edits an item and keeps it open, such as "trim the done steps out of this `human-task` and keep the rest": the session edits the item's `backlog/<slug>.md` on its spec branch to what the answer leaves, leaves the item out of `resolves`, and says in `## Intent` what it changed and that the item stays open, taking the backlog-only route when there is nothing else to design. Until now the prompt only knew answers that settle an item (it stays in `resolves`) and answers that drop it (deleted), so a draft following it would have deleted steps the human wanted kept.

## Problem

- `draft-answers` tells the session that a `question` or `human-task` item the answers settle stays in `resolves`, and the steps tell it to write `resolves` naming every item.
- A change deletes the items it resolves when it is archived (`task-deletes-fixed-items`, `archive-on-sync`). An answer that marks part of a `human-task` done and keeps the rest waiting on the human settles nothing, yet the prompt's only routes delete the item.
- A draft met exactly this and had to write `resolves: []` on its own judgment, against the prompt.

## Approaches

1. **Name the edit route in the prompt (chosen).** One more clause in the answers paragraph, beside the existing route for dropped items: edit the item on `spec/<id>`, leave it out of `resolves`, say so in `## Intent`. The branch then edits `backlog/`, which the backlog-only route (`draft-backlog-only-change`) already applies, gates and lands, and `clm draft --finish` already keeps the claim of an item the branch edits but doesn't resolve while the change waits. No new mechanism (`reuse-first`, `yagni`).
2. **Have triage edit the item itself, without a draft.** The triage session would trim the item in its own checkout. Rejected: triage never designs or edits in its own context except to revise a declined fold, and a backlog edit still needs a branch and a land, which the draft route already provides.
3. **Let a change resolve an item without deleting it.** A new `resolves` mode or key. Rejected: a second meaning for `resolves` to fix a prompt that simply omits a route.

## Outcome

- `orchestrator.triage/draft-answers` (modified): the steps say that an item the answers only edit, leaving it open, is left out of `resolves`, its `backlog/<slug>.md` edited on `spec/<id>`, with `## Intent` saying so, taking the backlog-only route when nothing else is to design. A new scenario checks the prompt's words.

## Edge cases

- An answer that edits an item and also settles a fact: the fact goes into the specs as today, and the item is still left out of `resolves` while it holds open steps.
- The item's claim: `clm draft --finish` keeps the claim of an item the waiting branch edits but doesn't resolve (`propose-claims`), so it isn't drafted again on its old text before the edit lands; a change that lands on checks releases it at once.
