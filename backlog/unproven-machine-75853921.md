---
title: Tests of machine notice no change to its code
kind: bug
components: [machine]
unproven: [machine/build-measure@25e6eaf6, machine/disk-room@6a2b07c4]
---

After the land `75853921` the mutation pass changed the code of `machine`, and the tests that cite these requirements passed with every change it tried:

- `machine/build-measure`: `crates/clm-machine/tests/edges.rs` `a_sparse_file_takes_the_blocks_it_has`, `crates/clm-machine/tests/measure.rs` `scenario_measured_and_kept`, `crates/clm-machine/tests/measure.rs` `scenario_due_again`, `crates/clm-machine/tests/measure.rs` `scenario_hard_links_once`, `crates/clm-machine/tests/measure.rs` `scenario_damaged_record`, `crates/clm-machine/tests/measure.rs` `a_place_without_a_worktree_is_unmeasured`, `crates/clm-machine/tests/measure.rs` `a_record_round_trips_and_its_peak_only_rises`, `crates/clm-machine/tests/measure.rs` `a_record_dated_in_the_future_is_due`, `crates/clm-machine/tests/measure.rs` `a_record_that_cannot_be_read_counts_as_none`
- `machine/disk-room`: `crates/clm-machine/tests/report.rs` `scenario_room_against_the_builds`, `crates/clm-machine/tests/report.rs` `scenario_no_room_before_any_measurement`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
