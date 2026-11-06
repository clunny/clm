---
title: "Draft prompt has no route for a settled question whose remaining work is code-only"
kind: bug
components: [orchestrator.triage]
issues: [163]
---

When the human answers a `question` item in the triage session and the answer means the only work left is code, with no requirement or design to change, the draft prompt gives the session no way through. Its answers paragraph keeps the settled question in `resolves` and asks for the fact to be written into the specs as a requirement or decision. Its code-fix step names only `bug` and `deferred` items, and `clm claim --fix` correctly refuses a question. A draft that met this case recorded the answer as a decision, deleted the question and filed a fresh `kind: bug` item for the same work. That bug could only be fixed in a later draft, so the human's answer cost two sessions.

Expected: the draft prompt (and the orchestrator.triage Design that states it) names one route for this case that finishes in a single draft. For example: the spec branch records the answer as a decision and resolves the question, so `clm apply` deletes it. The code work is then done in the same draft by the code-fix route, under an item the human's answer has made fixable in code. A build must still never close the question itself: `clm claim --fix` keeps refusing `question` items, as `apply-deletes-answered-questions` and `deferred-work-resolves-in-code` require, so only the human's answer, carried by a spec change, releases what the question blocked.
