---
title: "A component held by several verify-failure bugs admits a branch that fixes any one of them"
date: 2026-11-10
status: accepted
---

Resolves `backlog/two-verify-failure-bugs-on-one-component-block-each-other-s-fixes` (`changes/20261110-two-verify-failure-bugs/`, GitHub issue #486). Refines `verify-after-land`.

`land/verify-stop` holds a component while a `kind: bug` item with `verify_failures` names it. It admitted a code branch on that component only when the branch resolved every item holding it, so two items on one component, drafted and fixed apart, refused each other's fixes, and only a hand merge of both fixes onto one branch got either to `main`.

**A held component admits a branch that resolves any item holding it.** For each held component the branch's code touches, a commit it adds must carry `Resolves: backlog/<slug>` for at least one item that holds that component; the items it doesn't resolve stay on `main`'s backlog and go on holding the component for the next branch. The stop exists so that what lands on a component whose slow tests a human judged broken is the work that repairs them (`verify-after-land`). A branch resolving one of the items is that work, and the stop never judged a fix's contents beyond its trailer: a branch that resolves a single holding item may already carry other code on the component. So the refinement admits nothing the stop meant to keep out, and each item is still released only by its own fix. A component held by an item the branch doesn't resolve, and no other item it does, is refused as before.

Rejected:
- **Only a clearer refusal**, naming every holding item and saying one branch must resolve them all. Separate fixes still couldn't land, and the drafts that `clm triage` starts per group would still wait on each other.
- **Triage putting every `verify_failures` bug on a component into one draft.** Items that share one component but not the rest still split, it couples the triage of unrelated failures to stop a rule's misfire, and the land would still refuse a hand-made fix of one item (`yagni`).
