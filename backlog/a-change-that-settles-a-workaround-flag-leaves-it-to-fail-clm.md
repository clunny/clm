---
title: "A change that settles a workaround flag leaves it to fail clm check after clm apply"
kind: bug
components: [apply]
---

Step: a spec change whose `resolves` names a `kind: workaround` item and whose delta modifies the requirement the flag pins, then `clm apply <id>` and `clm check`.

What happened: `clm apply` deleted the change's resolved `question` and `reconsider` items but, as `apply/answered-questions` states, no item of another kind. The workaround flag still pinned the requirement's old version, so `clm check` failed on it (`model/parse-backlog`) until the session deleted the flag by hand in a commit of its own. Earlier changes did the same by hand; no instruction, skill or requirement says a session must.

Expected: one stated route for a change that settles a flag: either `clm apply` deletes a resolved workaround flag on a requirement the change modifies, or the check's message and the propose skill say to delete it in the change.

Rule involved: `apply/answered-questions`, `workaround-flags`, `redesign-before-workaround`.
