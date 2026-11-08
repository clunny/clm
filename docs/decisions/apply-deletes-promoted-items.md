---
title: "clm apply deletes the item a change is promoted from"
date: 2026-11-08
status: accepted
---

A change's `from: backlog/<slug>` names the item it was promoted from, and `docs/spec-format.md` said the item is deleted in the same spec PR. Nothing did it. The propose skill's numbered steps say to write `from` or `resolves` and to delete an item only when design finds it already fixed; only its "Elaborating an idea" section says to delete a promoted idea. `clm check` and the gate test only the shape of `from` (`bare-resolves-slugs`), and archiving deletes only the items `resolves` names (`archive-on-sync`). On 2026-11-08 a spec branch proposed from an idea landed with the idea still in `backlog/`, listed as open work, and a follow-up branch was needed to retire it.

Now `clm apply` deletes the item `from` names, whatever its kind, as one of the writes it holds back until the change validates (`apply/promoted-item`). Every route that lands a change runs `clm apply` on its spec branch (`propose`, a triage draft, the human), so the item leaves the backlog when the branch lands, with no step for an author to remember. A branch that already deleted the item by hand applies as before.

Extends `apply-deletes-answered-questions` from questions in `resolves` to the `from` item, for the same reason: the command can do what every author would otherwise do by hand.

Rejected:
- **Spell the deletion out in the propose skill's numbered steps.** The rule was already written in two places and was still missed; a third copy in prose is one more thing to remember, and triage drafts and hand-written changes don't read that skill.
- **A `clm check` rule that reports a live change whose `from` item is still in `backlog/`.** The same outcome, but every author must do by hand what the command can do, and an unattended draft that misses it fails late, as `apply-deletes-answered-questions` rejected for questions.
- **Write `resolves` instead of `from` for an idea the change takes up.** Archiving would delete the item only once the change is built, so it would stay listed as open work for the whole build, and `from` would have no use left.
