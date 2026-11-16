---
title: "An answered question still shows as waiting on the human"
kind: bug
components: [orchestrator]
---

Step: the human answers a `question` backlog item in a session, and the answer is to be carried out later by a spec change.

What happened: the session wrote the answer into the item's body. The item stayed `kind: question`, so `clm status` and the views went on listing it under what waits on the human, though nothing more was asked of them. The only ways to end that are deleting the item, a change that names it in `resolves`, or an `answered` record from the web app.

Expected: a session can record that a question is answered, so it leaves the list of what waits on the human while it still blocks the requirements that wait for the answer to be carried out.

Rule or decision involved: `blocked-by-backlog`, `web-answers-as-ledger-records`, `apply-deletes-answered-questions`.
