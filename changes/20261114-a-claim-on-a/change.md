---
status: applied
resolves: [a-claim-on-a-backlog-item-stays-live-after-its-change-lands-and]
---

## Intent

A spec change's land releases the backlog claims on the items its change names. Today a session that claims an item with `clm claim backlog/<slug>` and lands a change keeping the item in the backlog (one that narrows or defers it) leaves `propose/<slug>` on `origin`, so `clm proposals` and `clm status` show the item claimed for up to four more hours, and a later session has to ask the human whether a dead claim still holds it.

## Problem

`propose-claims` says the claim only has to last until the session's spec branch names the item; from then on the spec work holds it. Nothing ends the claim when that spec work lands. Two paths release claims today, and neither covers an attended session:
- `clm draft --finish` releases the claims a triage draft's place names (`orchestrator.triage`), but an attended `propose` session has no place.
- The propose skill's idea-elaboration steps tell the session to delete its claim by hand after the land; its steps for a change say nothing.

An item a landed change names in `resolves` stays held by *on main* until its fix deletes it, so its stale claim only clutters `clm proposals`. An item named only in `from`, the narrowing or deferring case, is held by nothing but the stale claim.

## Approaches

- **`clm land` releases the claims on the items its landed changes name** (taken). The land already reads the landed change IDs for its `landed` record, already deletes the landed `spec/*` branch through `host`'s `branch delete` leased on a tip, and knows which items the change names. Those are the items its spec work held, and `clm claim` refuses an item named by spec work in flight, so a live claim on one is the change's own session's, or a lapsed one nothing else holds. Leasing on the fetched tip keeps a claim another session took over in the meantime.
- **`clm wrap-up` releases them.** Not taken: wrap-up tidies the local clone, can't tell which claims the session took, and an unattended session may never run it.
- **The propose skill tells the session to delete its claims after the land.** Not taken: an instruction, not a check, and the instruction route already failed once; the land knows everything it needs.

## Outcome

- `land/landed-change-releases-claims`: after pushing a `spec/*` branch, `clm land` deletes `propose/<slug>` for each item its changes name in `from` or `resolves`, leased on the fetched tip, writing `released propose/<slug>` to stderr; a failed delete warns and never fails the land.

`clm draft --finish`'s release of a draft's claims stays: it also releases claims on items a draft left out of its change, and it already skips a claim that is gone. The propose skill's idea-elaboration steps keep their delete: an idea branch has no `change.md`, so the land has nothing to read for it.

## Constraints

- A session that claims an item and never names it in its change's `from` or `resolves` still leaves the claim to lapse; the propose skill already requires naming it.
- No edge cases or workarounds known.
