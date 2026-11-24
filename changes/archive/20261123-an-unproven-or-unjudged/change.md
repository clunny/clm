---
status: done
resolves: [an-unproven-or-unjudged-item-is-filed-again-once-git-lengthens-a]
---

## Intent

The filing of `unproven` and `unjudged` items (`verify/files-unproven`) finds an item it already filed by any name holding a prefix of the record's commit at least 7 characters long, as `kept-log` finds a kept log, so an item a build deleted is not filed again once git lengthens its abbreviations. Item names stay as `git rev-parse --short` gives them.

## Problem

`files-unproven` names each item `backlog/unproven-<component>-<short sha of R>.md` (and `unjudged-...` of `R'`) and skips filing when a commit after `R` added "a file of that path", the path built from today's abbreviation. git lengthens `rev-parse --short` as a repository's object count grows, so once it gives `R` more digits than when the item was filed, the check looks for a path nobody added and files the item again, bringing back one a build had deleted. `unproven` records stay in the ledger and the `unjudged` run is recomputed on every `clm verify --due`, so both kinds hit it.

## Approaches

- **Match by prefix (chosen).** Skip when a commit after `R` added `backlog/<kind>-<component>-<p>.md`, `<p>` a prefix of `R`'s full ID at least 7 characters long. It reuses the rule `kept-log` already follows for the same problem (`reuse-first`), keeps every item already filed in any repository recognised whatever length it was named at, and changes no name.
- **Name with a fixed length** (say 8 hex digits). Simpler to state, but in a repository whose items were filed with 7 digits, every deleted item of a record still in the ledger would be filed once more, the very bug, and the name would stop matching the abbreviation every other line of `clm verify` prints.

## Outcome

- `verify/files-unproven` states the prefix match for both kinds, and a scenario, `resolved under a shorter name`, files nothing when items named with 7 digits were added and deleted and `core.abbrev` is 12.
- The Design's **Filing** paragraph says why, pointing at `kept-log`.

## Constraints

- No name changes, so nothing else that reads item names (none does today) is affected.
- Edge case known: an item of the same component named after another commit `R2` whose ID shares its first 7 or more characters with `R`'s, added after `R`, would keep `R`'s item from being filed. git gave `R2` that name because it was unambiguous then, so this needs two commits of one chain agreeing on the digits git printed, each with a record of the same component: accepted rather than designed for (`yagni`).
