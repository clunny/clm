---
status: proposed
---

# A rename is an adding requirement and a removing one

## Intent

Renaming part of a Contract that other components use (a value of an exported type, a field, a function) can't be written as one requirement: its removal needs the users' moves, and their moves need the new name, so the needs form a cycle and the draft has to work out the route by hand. This change records the route as a decision, `rename-adds-then-removes`: an adding requirement that offers the new name beside the old and needs nothing, the users' moves needing it, and a removing requirement needing the moves, all in one change. The propose builder check and the `clm draft` prompt both say so. It comes from issue #61 (clunny/clm), accepted by the maintainer.

## Problem

`contract-removal-needs` speaks of a requirement that "removes or renames" contract, as if a rename were one step, and `contract-step-deferred` names only the cases where a removal can't carry its needs. A draft that renamed a value one component exports and three use found the cycle, designed a widened type that would have broken one user's exhaustive match on the provider's build (only the builder check caught it), then fell back on a deferred contract change, which a rename between required components doesn't need.

## Approaches

- **Split the rename into an adding and a removing requirement, in one change (chosen).** The needs run one way, the queue orders the builds, and nothing is left for the human to judge later. The adding requirement must leave users building, so the builder check searches for exhaustive uses.
- **Name the case in `contract-step-deferred` and use a deferred second change.** Rejected: it works, but it leaves a `proposed` change and a `deferred` item to remember for a case that needs within one change already order.
- **Let the provider's build edit its users for a rename.** Rejected: it breaks `architecture/path-ownership`; `clm run`'s compile-only fixes are for what a removal forces.

## Outcome

- `skills/propose/SKILL.md`'s builder check and the `clm draft` prompt's builder-check paragraph name `rename-adds-then-removes` and the split, and ask for a search of exhaustive uses.
- `contract-removal-needs` carries an amendment pointing at the new decision.

## Constraints

- `contract-step-deferred` stays as it is for the cases it names.
