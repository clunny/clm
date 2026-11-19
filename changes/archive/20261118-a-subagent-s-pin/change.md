---
status: done
resolves: [a-subagent-s-pin-moves-when-its-first-record-is-re-recorded]
---

## Intent

A subagent's pin stays on one branch however often its records are re-recorded. When the fold holds `effort` records of one `<session>/<agent>` for two branches, the branch that sorts first pins it, not the record with the earliest `at`, which moves each time a record is re-recorded.

## Problem

`effort/record-command` pins each `<session>/<agent>` to the branch of its `effort` record with the earliest `at`. The fold keeps only each `effort_id`'s latest record (`ledger/effort-records`), so the `at` compared is each branch's latest re-record, not when it was first recorded. With records for `impl/a` at t1 and, from another clone, `impl/b` at t2, a re-record of `impl/a` at t3 moves the pin to `impl/b`; the next run grows `impl/b`'s record, which moves it back, until both hold the whole of the subagent's work. The Design accepts the work being counted on both branches in this two-clone case, but as far as each clone saw it, and says the pin is fixed once recorded, which it isn't.

## Approaches

- **Taken: the branch that sorts first pins it.** Re-recording never changes which branches the records hold, so the pin can't move on a re-record, and every clone folds to the same pin. It is a change of one comparison in `effort`'s reading of the fold, and touches neither the ledger nor its records.
- **Not taken: the fold keeps the first `at` of each `effort_id`.** `clm ledger compact` drops superseded `effort` records, so after a compaction the fold can't know the first `at` anyway; holding it would need a new `effort` record field, a change to `ledger`'s Contract and `Body::Effort`, which `effort` builds by struct literal (`coupled-changes-sequence`), for a case that needs two clones recording one subagent.
- **Not taken: say the pin follows the latest records.** It makes the requirement true of the code as it is, but leaves the pin flipping, so the two-clone case counts the whole of the work twice instead of what each clone saw, and the Design's reason for a pin, that it is fixed once recorded, stops holding.

## Outcome

- `effort/record-command` (modified): between two records of one `<session>/<agent>`, the branch that sorts first by byte order pins it, whatever their `at`; a new scenario re-records the first record after the second and finds the pin unmoved. `unrecorded` reads with the same pins, as the requirement already says.
- `effort`'s Design says why an `at` can't decide the pin, and what stays accepted.

## Edge cases and workarounds

- A clone that records a subagent for a third branch before it has fetched the pin moves the pin when that branch sorts first, counting the work there too. It needs a race between clones, as the two-clone case does; accepted in the Design.
- A subagent the ledger already holds records of for two branches may have its pin move once, to the branch that sorts first, when this is built; no record is rewritten.
- `effort`'s test `the_earliest_record_pins_a_subagent` records `impl/b` before `impl/a` and expects `impl/b`; under the new rule `impl/a` pins, so the build rewrites it, as the new scenario's test or beside it, re-cited at the new version.
- No capability test sees a difference: a single clone's records keep their branch.

## Open questions

None.
