---
title: "A build that does a deferred item's work deletes it, named by a commit trailer"
date: 2026-10-31
status: accepted
---

`contract-removal-needs` has a build that can't remove an `obsolete` requirement's code, because another node still uses it, keep the code and file an item without `blocks`. The item was a `question`, and `code-fix-resolves-bug` let a code branch delete only `kind: bug` items, so when a later build of the node did remove the code, once the other node had moved off it, it couldn't delete the item: deleting made `clm gate` call the branch mixed. On 2026-10-31 two builds hit this and left settled items in the backlog for a spec branch to delete, a spec branch whose only work was the deletion.

Such an item asks nothing of the human. What it waits on is a fact about code, that the other node no longer uses what was kept, and the work it names is the provider's own `no-dead-code` cleanup. So it is filed as `kind: deferred`, work put off until a condition, and the `Resolves: backlog/<slug>` trailer that closes a bug closes it too: the gate's exemption covers `kind: deferred` as it covers `kind: bug`, under the same bounds (the branch deletes the item, and its code touches a node the item names, by `paths` or by a test's marker). The `clm run` prompt tells a build to file the kept-code item as `deferred` and to delete a `deferred` item naming its node when its branch does the work.

`clm triage` still drafts no `deferred` item (`backlog-triage`): one whose condition only the human can judge is never settled by a build's work, so the trailer changes nothing for it.

`question`, `human` and `idea` items stay out. Closing a question or a human step is the human's answer, and deleting one releases the requirements it blocks (`requirement-blocks`); closing an idea is choosing not to do it. A build that could delete them would answer for the human. A question filed with `blocks` because a dependant still uses what a requirement removes stays a question: a spec branch gives the requirement its `Needs` and deletes it.

Refines `code-fix-resolves-bug` (deferred items qualify as well as bugs) and `contract-removal-needs` (the kept-code item is `deferred`, and the build that removes the code closes it, instead of a spec PR).

Rejected:
- **Let the trailer close any kind.** A build could then delete a question that blocks its own requirements and build them, answering for the human.
- **Let the trailer close a `question` with no `blocks`.** A question without blocks is still asking the human something (a deploy, an account); `blocks` says what waits, not whether an answer is needed.
- **Keep the item a question and leave its deletion to a spec branch.** What happened twice: a spec branch, and a session drafting it, for one deletion.

**Amended 2026-11-01** (`20261101-triage-owns-friction-deferred`). `clm triage` now prints a `deferred` line per untaken deferred item, and the `triage` skill shows each to the human, who judges its condition and has it drafted, kept or dropped (`triage-owns-backlog-work`). It is still never drafted unless the human asks.
