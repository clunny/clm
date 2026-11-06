---
title: "The regenerate applies a change after the changes whose output its bases record, and otherwise in ID order"
date: 2026-11-06
status: accepted
---

`clm rebase`'s regenerate and `clm land`'s derived-specs check applied a branch's changes in ID order (`regenerate-applied-specs`). When one spec branch carries two applied changes and the second's deltas record bases taken from the specs the first produces, a dependent change whose ID sorts first met a tree without the other's edits: its bases were found at no stage and in no commit of `main`'s history, so it was refused as stale, the merge had no base text to work with, and the report said `no text for <element>`. The only way out was to rename the dependent change so its ID sorted later (issue 204), so the ID, which `clm draft` or a session picks from a date and a slug, carried meaning nobody had stated.

So the regenerate picks the next change by its bases: of the changes left, the first in ID order none of whose recorded bases is *unknown*, an unknown base being one for an element the change MODIFIES or REMOVES that no stage of the tree so far has and that no commit of `onto`'s history of the component's `SPEC.md` has, which is the text `stale-delta-diff` already looks for. A base another change on the branch produces is unknown until that change is applied, so the dependent change waits for it. When every change left has an unknown base, the first in ID order goes next and is merged and reported as before. Changes none of which waits on another (every fold, whose drafts all base on `main`) are applied in ID order as before, so the movers, the `Merge` commits and the fold prompt read as they did.

Alternatives considered:
- **The order the branch applied them in.** Read from the commits that set each `status: applied`. A change set back to `proposed`, edited and applied again, an archive move, and a fold that brings each change over as one commit all blur that order, and `clm land` would need the same history walk on the merge commit. The bases already say which change builds on which.
- **An order the changes declare** (an `after:` key in `change.md`). A new frontmatter key, a validation and a glossary entry, and a session that forgets it hits the same stale report; the regenerate can read the dependency from the bases the session already records.
- **Keep ID order and tell the propose skill to name changes to sort.** The guidance would be a workaround for a rule the tool can follow itself (`redesign-before-workaround`).
- **Try every order until one applies.** Exponential in the changes on the branch, and a stale change stays stale in every order; picking by known bases is one pass per change.

Refines `stale-delta-names-movers` and `regenerate-applied-specs`: the regenerate no longer applies a branch's changes in ID order when one change's bases build on another's; the movers are still taken from the stages.
