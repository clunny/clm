---
title: "check_env's a_record_under_other_variables fails when its two checked records share a second"
kind: bug
components: [land]
---

`crates/clm-land/tests/check_env.rs` `a_record_under_other_variables` failed in `clm land --check-only` on `impl/clm` (2026-11-17) and passed three times alone:

    left: "/tmp/.tmpVZ8cg9/bin/check"
    right: "A=1 /tmp/.tmpVZ8cg9/bin/check"

The test writes a `checked` record with `mark_checked`, lands, and asserts that `checked_commands().last()` is the land's record. `clm_ledger::fold` sorts records by `(at, id)` (`crates/clm-ledger/src/fold.rs`), so when both records fall in the same second of `at` their order is the order of their ids, and `last()` can be the fixture's record. The test should find the land's record by what it names (for example the record whose command differs from the fixture's, or the one for the landed code key), not by its position.
