---
title: Tests of vcs notice no change to its code
kind: bug
components: [vcs]
unproven: [vcs/one-git-layer@672cd7c7]
---

After the land `75853921` the mutation pass changed the code of `vcs`, and the tests that cite these requirements passed with every change it tried:

- `vcs/one-git-layer`: `crates/clm-git/tests/scan.rs` `a_wrapper_comes_back`, `crates/clm-git/tests/scan.rs` `a_crate_that_has_not_migrated`, `crates/clm-git/tests/scan.rs` `clean_dependants`, `crates/clm-git/tests/scan.rs` `the_azure_cli`, `crates/clm-git/tests/scan.rs` `a_wrapper_that_takes_the_program`, `crates/clm-git/tests/scan.rs` `a_call_split_across_lines`, `crates/clm-git/tests/scan.rs` `a_crate_that_runs_other_programs`, `crates/clm-git/tests/scan.rs` `the_workspace_runs_git_only_through_the_layer`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
