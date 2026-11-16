---
title: "Five listed tests stay in the root, raised as a question"
kind: workaround
components: [land]
requirement: land/commands-in-crate@6ed92c77
---

Five of the 15 tests moved-tests.txt lists for land still run the clm binary from crates/clm/tests/land.rs: check_deletes_the_binary and copy_removed (the land copies its own executable and runs the real gate and clm check from it; called in a test, the executable is the test binary) and overlapping_lands, a_callers_target_takes_the_lock_too and held_through_the_gate_on_the_merge (two clm land processes at once, one with its own CARGO_TARGET_DIR). The requirement's last sentence says none runs the binary; backlog/five-land-tests-observe-only-the-clm-process.md asks for the spec change and blocks clm/routes-only.
