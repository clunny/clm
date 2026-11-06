---
title: "clm rebase and clm land re-apply a branch's changes by ID, so a change built on another must sort after it"
kind: bug
components: [sync, sync.regenerate, land]
issues: [204]
---

When one spec branch carries two changes and the second's deltas record bases taken from the specs the first change produces, the regenerate step in `clm rebase` (and the same tree `clm land`'s derived-specs check builds) applies them sorted by change ID. If the dependent change's ID sorts first, its deltas meet a tree without the other change's edits, and they are reported stale even though nothing on `main` moved. Today the only way out is to rename the dependent change so its ID sorts later.

Expected: the regenerated `specs/` applies a branch's changes in an order that respects which builds on which, such as the order the branch applied them in, or an order the changes declare, so a change's ID carries no meaning. `sync/reapply`, `sync.regenerate`'s Contract and `regenerated-tree`, and `land/derived-specs` all state ID order today, so this starts on a spec branch, and the movers and fold rules that read "earlier in ID order" (`stale-delta-names-movers`) need the same new order. If ID order stays, the propose skill, which already tells a session to put a deferred contract step in a second change on the same branch, should say that the changes are re-applied by ID and how to name them.
