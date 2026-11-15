---
title: Tests of upgrade notice no change to its code
kind: bug
components: [upgrade]
unproven: [upgrade/forms-without-a-list-gone@991edcce]
---

After the land `36890663` the mutation pass changed the code of `upgrade`, and the tests that cite these requirements passed with every change it tried:

- `upgrade/forms-without-a-list-gone`: `crates/clm-upgrade/tests/forms_gone.rs` `no_form_without_a_list_is_exported_called_or_marked`, `crates/clm-upgrade/tests/forms_gone.rs` `a_left_form_is_found_by_name_and_a_form_with_a_list_is_not`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
