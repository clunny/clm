---
title: "land test held_through_the_gate_on_the_merge fails when its two lands are taken together"
kind: bug
components: [clm]
---

`crates/clm/tests/land.rs`'s `held_through_the_gate_on_the_merge` (cites `land/land-target-lock`) starts two `clm land` processes at once and expects the log `gate, check, gate, gate, check, gate`. Since `land/batched-lands`, a land that finds the other waiting as it takes the land lock lands both on one check, so the log is then `["gate", "gate", "check impl/a"]` and the test panics. The window is the instant between the first land taking the lock and listing the waiting lands, so it is rare, and likelier on a loaded host. Reproduce: add a 1.5 s sleep before `lock::waiting_lands` in `crates/clm-land/src/batch.rs` `run`, and the test fails every time. Fix as `overlapping_lands` in the same file and `clm-land`'s `a_land_with_the_callers_target` do: start the second land once the first's check has logged its start. Found while fixing `backlog/check-lock-test-a-land-with-the-callers-target-fails-when-other`, the same race in `clm-land`; this file belongs to the root component, so that fix could not carry it.
