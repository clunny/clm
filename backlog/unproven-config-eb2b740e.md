---
title: Tests of config notice no change to its code
kind: bug
components: [config]
unproven: [config/unreferenced-detector@f0c53e15]
---

After the land `eb2b740e` the mutation pass changed the code of `config`, and the tests that cite these requirements passed with every change it tried:

- `config/unreferenced-detector`: `crates/clm-config/tests/this_repository.rs` `unreferenced_detector_is_named_here`, `crates/clm-config/tests/this_repository.rs` `unreferenced_allow_list_gives_every_entry_a_reason`, `crates/clm/tests/cli.rs` `unreferenced_is_clean_with_the_allow_list`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
