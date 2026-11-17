---
title: Tests of plugin.hooks.guard notice no change to its code
kind: bug
components: [plugin.hooks.guard]
unproven: [plugin.hooks.guard/block-refuses-whole-command@b3879746, plugin.hooks.guard/commands-in-crate@11cfbd2d, plugin.hooks.guard/defect-approval@4c044d8e, plugin.hooks.guard/guard-per-agent-harness@b6263bcf, plugin.hooks.guard/hook-budgets@0a955c81]
---

After the land `eb2b740e` the mutation pass changed the code of `plugin.hooks.guard`, and the tests that cite these requirements passed with every change it tried:

- `plugin.hooks.guard/block-refuses-whole-command`: `crates/clm-guard/tests/guard.rs` `the_hook_judges_with_the_hooks_budgets`
- `plugin.hooks.guard/commands-in-crate`: `crates/clm-guard/tests/guard.rs` `the_hook_judges_with_the_hooks_budgets`
- `plugin.hooks.guard/defect-approval`: `crates/clm-guard/tests/guard.rs` `the_hook_judges_with_the_hooks_budgets`
- `plugin.hooks.guard/guard-per-agent-harness`: `crates/clm-guard/tests/guard.rs` `the_hook_judges_with_the_hooks_budgets`
- `plugin.hooks.guard/hook-budgets`: `crates/clm-guard/tests/guard.rs` `the_hooks_budgets_are_twenty_and_thirty_seconds`, `crates/clm-guard/tests/guard.rs` `nothing_shortens_the_hooks_budgets`, `crates/clm-guard/tests/guard.rs` `the_hook_judges_with_the_hooks_budgets`

A test that proves the requirement fails when its code is wrong: strengthen these so one does, or, when they are already sound, delete this item.
