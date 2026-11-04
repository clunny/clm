---
title: "A rename of contract other components use is an adding requirement and a removing one, in one change"
date: 2026-11-04
status: accepted
---

`contract-removal-needs` says a requirement that removes or renames part of a Contract another component's code uses needs the requirements that move each such component off it. Written as one requirement, a rename can't be ordered: the removal needs the users' moves, and the users' moves need the new name, which only the provider's build adds, so the needs form a cycle, which the model rejects. Each build may touch only its own component, and the land typechecks the whole workspace, so neither side can go first. A draft that met this (a value of a type one component exports and three others use) worked out the pattern by hand, after a first design whose widened type would have broken a user's exhaustive match on the provider's build.

Now a rename is written as two requirements of the provider, in the same change:

- an **adding** requirement that offers the new name beside the old and needs nothing. It must leave every user's code building as it stands: a second name (a new value, field, type or function) that users don't yet read, never a widened type that a user matches exhaustively, so the provider's build breaks no one;
- a **removing** requirement that withdraws the old name, stated as an added or modified requirement (not `REMOVED Requirements`) so it can carry a `#### Needs` naming each user's move.

Each user's move is a requirement of that user that needs the adding requirement. The adding requirement's build marks the old name `clm:remove-by <change-id>`, naming the change, which stays live until the removal is built (`expand-contract`, `check/remove-by-markers`). The needs then run one way (add, then moves, then removal), the queue builds them in that order, and no second change or deferred item is needed. The builder check asks the subagent to search each user's code for exhaustive uses (a `match`, a `switch` with no default, a `Record` keyed by the type) that the adding requirement must not break.

`contract-step-deferred` still applies when the removal can't carry its needs at all (a required provider waiting on an optional user, or a removed whole requirement); a rename between required components isn't such a case.

Refines `contract-removal-needs` and `expand-contract`.

Rejected:
- **A deferred contract change for every rename.** It works, but it leaves a change `proposed` and a `deferred` item for the human to judge, when needs within one change order the same steps with nothing to remember.
- **Allow a build to edit its users' code for a rename.** `architecture/path-ownership` keeps each build inside its component; `clm run`'s compile-only fixes cover only what a removal forces, not a move to a name the users choose to read.

(Amended 2026-11-04, `20261104-clm-run-allows-compile`: `clm run`'s compile-only fixes now cover what an addition forces too (`forced-fixes-cover-additions`), so the rejected option above reads "cover only what a removal or an addition forces". A rename's adding requirement that adds a struct field may rely on that fix, setting the field in users' struct literals to the value under which the provider behaves as before, since their behaviour stays as it stands; a user's move to the new name is still never such a fix.)
