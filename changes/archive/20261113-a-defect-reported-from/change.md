---
status: done
resolves:
  - a-defect-reported-from-the-build-session-stays-an-uncommitted
---

## Intent

Delete the backlog item `a-defect-reported-from-the-build-session-stays-an-uncommitted`, accepting the behaviour it reports. No spec, code or skill text changes.

The behaviour: `clm defect`, run by the session that drives `clm build` in the main checkout on `main`, writes the report as a new file under `backlog/` and commits nothing. The build skill names no step that carries the file to `main`, so no draft sees the item until a session lands it.

It is accepted for these reasons:

- **The freeze.** The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`) settles that, until it is done, the build loop and triage work only on the plan, that defects are filed and their default answer is "accept", and that an exception is the human's to make. This item is `P5` and blocks nothing, so nothing argues for an exception.
- **The report isn't lost.** The file stays in the main checkout's working tree, where `git status` and `clm backlog` show it, until a session carries it.
- **A route to `main` exists and is written down.** A branch that only adds a backlog item gates `code` and lands on checks (`gate/backlog-additions`, `small-change-no-claim`), and a spec branch may carry a new item too. That is how this item reached `main`: it is one of the two reports the spec branch of `20261113-land-starvation` carried.
- **Either fix runs against the plan.** One fix is a sentence in the build skill with a requirement whose test reads the sentence, which is the kind of requirement the plan deletes ("prompt-text requirements are deleted with their tests"). The other is new code in the build loop or `clm defect` that lands items by itself, which is work outside the plan, for a case met once.

What is given up: a defect the build session reports isn't drafted in the same drain. It waits until a session lands its item.
