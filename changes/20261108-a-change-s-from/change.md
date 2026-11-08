---
status: applied
resolves:
  - a-change-s-from-item-outlives-the-change-unless-the-session
---

## Intent

Make `clm apply` delete the backlog item a change's `from` names, so a change promoted from an item retires it on its own spec branch, whoever wrote the change. Today the rule that a promoted item is deleted in the same spec PR is written down, but nothing does it, and a session that forgets leaves the item listed as open work after its change has landed.

## Problem

`from: backlog/<slug>` names the item a change grew from. `docs/spec-format.md` says that item is deleted in the spec PR that promotes it, and `check/priority-caps` relies on it. But:

- the propose skill's numbered steps never say to delete it; only the "Elaborating an idea" section's last bullet does;
- `clm check` tests only the shape of `from` (`check/backlog-entry-shape`), not whether the item is gone;
- archiving deletes only the items `resolves` names (`archive-on-sync`).

So a change proposed from an idea landed on 2026-11-08 with the idea still in `backlog/`, and a follow-up branch had to retire it. Left there, a later session may propose the same idea again.

## Approaches

- **`clm apply` deletes the `from` item (chosen).** `clm apply` already deletes the questions a change's `resolves` names (`apply/answered-questions`), as a held-back write of the all-or-nothing apply. Every route that lands a change (`propose`, triage drafts, a human) runs it on the spec branch, so the item goes with no step to remember. It reuses the mechanism and needs no skill or check edit (`reuse-first`).
- **Spell it out in the propose skill's numbered steps.** The rule was already written twice and still missed; triage drafts and hand-written changes don't read that skill.
- **A `clm check` rule reporting a live change whose `from` item exists.** Same outcome, but it makes every author do by hand what the command can do, and fails late in an unattended draft. `apply-deletes-answered-questions` rejected the same option for questions.
- **Use `resolves` for an idea the change takes up.** The item would stay listed until the change is built and archived, and `from` would have no use left.

## Outcome

- `apply` gains `promoted-item`: `clm apply` deletes `backlog/<slug>.md` for the change's `from`, whatever the item's kind except `human-task`, printing `removed backlog/<slug>.md`, held back when the change has a violation, nothing when the item is already gone, once when `resolves` names it too, and nothing on `clm rebase`'s re-apply. Its Contract names the deletion.
- `docs/spec-format.md` says `clm apply` deletes the promoted item.
- Decision: `apply-deletes-promoted-items`.

## Constraints and edge cases

- An item the branch already deleted by hand is no violation, so existing skill text ("the item deleted on the spec branch") stays true and needs no edit.
- `clm proposals` reads `from` from the spec branch's `change.md`, not the item, so the item stays held by the change after `clm apply` deletes it and until the branch lands.
- Every `kind` but `human-task` is deleted: `from` means the change took the item over, unlike `resolves`, where `answered-questions` deletes only questions. A `human-task` item is a step only the human takes, and deleting it releases the requirements it `blocks`, so it stays until the human deletes it, as `apply-deletes-answered-questions` keeps one (the architecture audit's finding).
- A malformed `from` deletes nothing; `check/backlog-entry-shape` already reports it.
