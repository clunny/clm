---
title: "A question whose answer leaves only code is turned into a bug by the spec branch"
date: 2026-11-05
status: accepted
---

Sometimes the human answers a `question` item in the triage session, and the answer leaves only code to change. A requirement or scenario as written already states what the code must do, and none of the requirements the question blocks is waiting to be built for it. Until now the draft prompt gave this case no route that finished. A settled question stayed in `resolves` and its fact went into the specs. The code-fix route took only `bug` and `deferred` items, and `clm claim --fix` refuses a question. On 2026-11-05 a draft recorded the answer as a decision, deleted the question and filed a new `kind: bug` item for the same work (issue #163). Only a later draft could fix that bug, so the answer cost two sessions.

Now the draft's spec branch turns the item into a bug: `kind: bug`, no `blocks`, `components` where the fix lies, and a body that names the requirement the fix restores and quotes the answer. The item leaves `resolves`, and the branch takes the backlog-only route. Once `clm land` has landed it, the same draft claims the component with `clm claim <component> --fix backlog/<slug>` and fixes the bug, with a `Resolves:` trailer, as it fixes any code-only bug (`code-fix-resolves-bug`, `code-fix-needs-a-written-rule`). When the branch needs review, the draft stops as any does. The bug lands with the fold and reaches the next `clm triage`.

The question is still closed only by the human's answer, carried by a spec change. The build closes a bug, which is what the spec change made the item. `clm claim --fix` keeps refusing `question` items (`apply-deletes-answered-questions`, `deferred-work-resolves-in-code`).

Rejected:
- **Resolve the question and file a new bug.** This is what the draft did. A new slug is unclaimed between the land and the fix, so another triage may draft it too, and it loses the item's `issues`, which `clm intake` reads to report the issue resolved.
- **Let `clm claim --fix` take a question once triage has recorded an answer.** The issue suggested this. A build could then delete a question that blocks its own requirements, answering for the human, as `deferred-work-resolves-in-code` rejects. It would also need a record of the answer that the command can read.
- **Do the code fix before the spec branch lands.** No item on `origin/main` would allow the fix until the spec branch lands, since `clm claim --fix` reads the item there.
