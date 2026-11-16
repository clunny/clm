---
status: applied
resolves: [an-answered-question-still-shows-as-waiting-on-the-human]
---

## Intent

When the human answers a `question` item but its work has to wait, such as for other builds or another change to land first, the item stops showing as waiting on the human and still blocks the requirements the answer's work is for. The spec branch that takes the answer turns the item into a `deferred` item that quotes the answer and names the condition. The draft prompt names this route beside the two it already has. The decision `answered-question-waits-as-deferred` records the choice.

## Problem

- A session that took a human's answer to a `question` whose work waits could record it only in the item's body. The item stayed `kind: question`, so `clm status`, the views and the web app went on listing it as waiting on the human.
- The draft prompt's answers step names two routes for a settled question: it stays in `resolves` and `clm apply` deletes it (`apply-deletes-answered-questions`), or it becomes a bug when only code is left (`settled-question-becomes-bug`). Both assume the work can be done now. For work that waits, deleting the item releases the requirements it blocks before the work is done.
- The change `20261115-tests-that-stay-wait-on-no-one` turned seven such questions into `deferred` items by hand. No rule names that route, so the next session has to work it out again.

## Approaches

1. **Make the item `kind: deferred` (chosen).** `deferred` already means work that waits on a condition. `clm status`, the views and the web app list only `question` and `human-task` items as waiting on the human, `blocks` holds for an item of any kind (`requirement-blocks`), and `clm triage` offers a deferred item to the human to judge when it is due (`triage-owns-backlog-work`). No code reads anything new. Only the draft prompt's words change.
2. **An `answered` field on the question that the views read.** Rejected: the model, the views, the web app and `clm triage` would each learn a new state of an item that the existing kind already expresses (`reuse-first`, `yagni`).
3. **A session writes an `answered` ledger record, as the web app does.** Rejected: the record only carries the answer to the next triage, which drafts the item. The item stays a `question` and is still listed as waiting on the human.

## Outcome

- `orchestrator.triage/draft-answers` (modified): with answers, the prompt's steps name the route. A `question` item whose answer settles it but whose work waits leaves `resolves`. On `spec/<id>` its `kind` becomes `deferred`, its `choices` are removed, its `blocks` are kept, and its body quotes the answer, names the condition, and says that the change carrying the answer out names the item in `from`. The branch takes the backlog-only route. A new scenario checks the prompt's words. Without answers, the prompt doesn't name the route.
- The Design's **Answers** paragraph states the route and why it holds.
- `docs/spec-format.md`'s backlog section names the route for any session, not only a draft.

## Constraints

- The route is for an answer whose work can't be designed yet. An answer whose work can be designed now still goes into the change, and the question stays in `resolves`.
- The item keeps its slug, its `issues` and its `priority`, as a question turned into a bug does.

## Edge cases

- **The change that carries the answer out modifies a requirement the deferred item blocks.** In `resolves` alone, the item would be deleted only when the change is archived, and the change can't be archived while the requirement is blocked. So the item's body tells that change to name the item in `from`, and `clm apply` deletes it when it applies the change (`apply/promoted-item`), releasing what it blocks when the change lands.
- **One change carries out the answers of several deferred items.** `from` names one item, so the change names one there and deletes the others on its own spec branch, listing them in `resolves`. Each is then gone when that branch lands, as the `from` item is. The architecture audit raised this case.
- **The answer's work turns out to be code-only once the condition is met.** The deferred item is then fixed by component with `clm claim <component> --fix backlog/<slug>` and a `Resolves:` trailer (`deferred-work-resolves-in-code`), which deletes it and releases its blocks.
