---
title: "Two listed tests stay in the root, raised as a question"
kind: workaround
components: [config]
requirement: config/commands-in-crate@ecba0654
---

Two of the 38 tests moved-tests.txt lists for config still run the clm binary from crates/clm/tests/cli.rs: check_fails_on_a_retired_term_in_a_spec (asserts clm check's detector line) and unreferenced_is_clean_with_the_allow_list (runs clm unreferenced, the scenario names the binary under test). clm-config depends on nothing, so its tests can neither call those components' crates nor run the binary. The requirement's last sentence says none runs the binary; backlog/two-config-tests-run-other-components-commands.md asks for the spec change and blocks clm/routes-only.
