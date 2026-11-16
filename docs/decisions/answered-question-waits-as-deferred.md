---
title: "An answered question whose work waits becomes a deferred item"
date: 2026-11-15
status: accepted
---

Sometimes the human answers a `question` item, but the answer can't be carried out yet: it waits on other builds or on another change landing first. A session then had no way to record the answer except in the item's body. The item stayed `kind: question`, so `clm status`, the views and the web app went on listing it as waiting on the human, though nothing more was asked of them. Deleting it, naming it in a change's `resolves` or an `answered` record from the web app were the only ways out, and each either released the requirements it blocked before the answer's work was done or still listed it as waiting.

Now the spec branch that takes the answer turns the item into a `deferred` item. It sets `kind: deferred`, removes `choices`, keeps `blocks`, and rewrites the body to quote the answer, name the condition and say that the change carrying the answer out names the item in `from`. The item leaves `resolves`. `clm status` and the views list only `question` and `human-task` items as waiting on the human, so the item leaves that list, while its `blocks` still hold the requirements the answer's work is for (`requirement-blocks`). `clm triage` offers it as a `deferred` line, and the human judges when the condition is met (`triage-owns-backlog-work`). The change that then carries the answer out names the item in `from`, so `clm apply` deletes it and releases what it blocks when that change lands (`apply/promoted-item`). In `resolves` alone, a deferred item is deleted only when the change is archived, which a requirement the item blocks would never let happen.

The `20261115-tests-that-stay-wait-on-no-one` change did this by hand for seven questions; this records it as the route, and the draft prompt names it (`orchestrator.triage/draft-answers`).

Refines `apply-deletes-answered-questions`, `settled-question-becomes-bug` and `web-answers-as-ledger-records`. An answer whose work can be designed now stays in `resolves`, and `clm apply` deletes the question; an answer that leaves only code makes the item a bug; an answer whose work waits makes it deferred.

Rejected:
- **An `answered` field on a question that the views read.** The model, the views, the web app and `clm triage` would each learn a new state of an item, where the existing `deferred` kind already means work that waits on a condition (`reuse-first`).
- **A session writes an `answered` ledger record, as the web app does.** A record only carries the answer to the next triage, which drafts the item; it doesn't say the item waits on a condition, and the item stays a `question`, listed as waiting on the human.
- **Delete the question and file a new deferred item.** The new slug loses the item's `issues` and claim, as `settled-question-becomes-bug` rejected for bugs.
