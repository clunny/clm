---
title: "The root keeps a test of sync's skipped-proposal warning"
kind: deferred
components: [clm, sync]
---

`clm/rebase-warnings-prefixed` was removed by `20261117-scenarios-only-a-process-shows`, and its scenario "a skipped proposal" moved to `sync/warnings-name-the-command`, whose call now writes the `clm rebase: ` prefix. `clm`'s build took the obsolete marker off `scenario_a_skipped_proposals_warning_names_the_command_as_run` in `crates/clm/tests/sync.rs` but kept the test, unmarked, since until `sync`'s build lands no test in `crates/clm-sync/` proves that a skipped proposal's warning reads `clm rebase: skipped proposal ...`, and deleting it would leave the scenario unproven.

The test goes once `sync`'s crate proves the scenario at `sync/warnings-name-the-command`'s current version: `clm`'s next build after that deletes it (`no-dead-code`).
