---
status: applied
resolves: [draft-prompt-settled-question-code-only]
---

## Intent

Sometimes the human answers a `question` item in the triage session and the answer leaves only code to change. The draft prompt now gives that case one route that finishes in the same draft. The draft's spec branch turns the question into a `kind: bug` item that carries the answer. Once that branch lands, the draft fixes the bug by component, as it fixes any code-only bug. A build still never closes a question, since `clm claim --fix` keeps refusing them. Only the human's answer, carried by the spec change, makes the item fixable in code. The rest of the change sets out why this route was chosen over the others.

## Problem

- `draft-answers` keeps a settled question in `resolves` and puts the fact it waited on into the specs. The code-fix route (`code-fix-resolves-bug`, `deferred-work-resolves-in-code`) takes only `bug` and `deferred` items, and `clm claim --fix` refuses a question.
- A draft whose answer left only code work recorded the answer as a decision and deleted the question. It then filed a new `kind: bug` item for the same work, which only a later draft could fix. The human's answer cost two sessions (issue #163).

## Approaches

1. **The spec branch turns the question into a bug, and the same draft fixes it once the branch lands (chosen).** The branch edits the item, and the backlog-only route (`draft-backlog-only-change`) and the code-fix route (`code-fix-resolves-bug`) already exist. Only their order is new: the fix comes after the spec land, since `clm claim --fix` reads the item on `origin/main`. The item keeps its slug, its claim and its `issues`.
2. **Resolve the question, file a new bug and fix it after the land.** This finishes in one draft too, but the new slug is unclaimed between the land and the fix, and the `issues` link that `clm intake` reads is lost with the deleted item.
3. **Let `clm claim --fix` take a question once triage has recorded an answer (the issue's suggestion).** Rejected: a build could then close a question that blocks its own requirements (`deferred-work-resolves-in-code`, `apply-deletes-answered-questions`), and the command would need a record of the answer that it can read.

The decision `settled-question-becomes-bug` records the choice.

## Outcome

- `orchestrator.triage/draft-answers` (modified): the steps name the route. A `question` item the answers settle while leaving only code to change leaves `resolves`. The draft turns it into a bug on `spec/<id>`, with `kind: bug`, no `blocks`, `components` where the fix lies, and a body that names the requirement the fix restores and quotes the answer. Once `clm land spec/<id>` has landed the branch, the draft fixes it with `clm claim <component> --fix backlog/<slug>` and a `Resolves:` trailer. A new scenario checks the prompt's words.
- The Design's **Answers** paragraph states the route and why it was chosen.

## Constraints

- The route applies only when a requirement or scenario as written already states what the code must do (`code-fix-needs-a-written-rule`). Otherwise the answer is designed into the specs as before.
- A question whose remaining work is building the requirements it blocks stays in `resolves`. `clm apply` deletes it, and the normal builds do the work.

## Edge cases

- **The spec branch needs review.** The draft stops as any does, and the item's claim is kept while the branch waits (`propose-claims`). The bug lands with the fold and reaches the next `clm triage`, which drafts it as an ordinary code-only bug.
- **The fix claim is refused after the land.** The bug stays on `main`, and the draft leaves it with `left <slug>: <reason>`. Its final line is still `landed <sha>` for the spec land.
- **The fix lies in components the question didn't name.** The branch sets the bug's `components` to those, so the claim takes them, since `clm claim --fix` takes a component that the item on `origin/main` names.
