---
title: Tests of init notice no change to its code
kind: bug
components: [init]
unproven: [init/own-migrations-frozen@13e2d029]
---

After the land `d021e717` the mutation pass changed the code of `init`, and the tests that cite these requirements passed with every change it tried:

- `init/own-migrations-frozen`: `crates/clm-init/tests/own_migrations_frozen.rs` `the_crates_own_migrations_match_its_lock`, `crates/clm-init/tests/own_migrations_frozen.rs` `a_crate_with_no_lock_file_passes`, `crates/clm-init/tests/own_migrations_frozen.rs` `a_shipped_migration_edited_is_reported`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
