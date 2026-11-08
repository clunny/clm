---
title: "A change's from item outlives the change unless the session remembers to delete it"
kind: bug
components: [plugin, check]
---

Step: `/clm:propose` with a backlog `idea` item as the intent, the change naming it in `from`.

What happened: the spec branch landed with the item still in `backlog/`. Steps 1 and 4 of the propose skill say to write `from` or `resolves` naming the items, and say to delete an item on the spec branch only when design finds it already fixed. Only the "Elaborating an idea" section's last bullet says a change proposed from an idea deletes the item on the spec branch. `clm check` and `clm gate` didn't flag a live change whose `from` item is still in the backlog, and archiving deletes only the items `resolves` names. So the item stays listed as open work after its change has landed, and a later session may propose it again. A follow-up branch was needed to retire it.

Expected: either the numbered steps say when a `from` item is deleted on the spec branch (or say to use `resolves` for an idea the change fully takes up), or a check reports a `from` item that outlives its change's landing.

Rules involved: `propose-claims`, `archive-on-sync` (which deletes only `resolves` items), `elaborate-ideas-on-main`.
