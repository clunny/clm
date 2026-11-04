---
title: "clm apply deletes the questions its change resolves"
date: 2026-10-31
status: accepted
---

`requirement-blocks` releases a blocked requirement when its item is deleted, and `blocked-by-backlog` has the human answer a question "by deleting the item in a small spec PR". A change that answers a question lists it in `resolves`, and `resolves` items are deleted by the build that fixes them or when `clm sync` archives the complete change (`task-deletes-fixed-items`, `archive-on-sync`). For a question that `blocks` a requirement the change modifies, that order deadlocks: the change completes only once the requirement is built, the requirement can't be built while the item exists, `clm next` hides the node, `clm claim` refuses it, and `clm claim --fix` takes only bugs. On 2026-10-31 the only way out was a separate spec branch that deleted the item and landed first.

Now `clm apply` deletes each `kind: question` item its change's `resolves` names, as one of the writes it holds back until the change validates (`apply/answered-questions`). The spec it applies is the answer, so the question is closed on the same spec branch, whoever drafted it (`propose`, a triage draft or the human), and the requirements it blocked are released when that branch lands.

Only questions. A `human` item is a step the human takes outside the spec, such as making an account; the change that relies on it can't take it, so the item stays until the human deletes it. A `bug`, `idea` or `deferred` item is closed by the work that fixes it, which the build or the archive still does.

Refines `task-deletes-fixed-items` and `archive-on-sync`: a resolved question is deleted at apply, before any build. Follows `blocked-by-backlog`: the spec branch that answers a question deletes it.

Rejected:
- **The queue treats an item a live change resolves as not blocking.** It would release a `human` item listed in `resolves` before the human took the step, needs the rule in the queue, the views and the web app alike, and leaves answered questions in the backlog where triage shows them as waiting on the human.
- **A gate or check rule that makes the spec branch delete the question.** The same outcome, but every author must do by hand what the command can do, and an unattended draft that misses it fails late.
- **Let `clm claim --fix` take a question.** A build could then delete a question that blocks its own requirements, answering for the human (`deferred-work-resolves-in-code`).
