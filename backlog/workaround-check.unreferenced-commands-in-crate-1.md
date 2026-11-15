---
title: "Two listed tests stay in the root, raised as a question"
kind: workaround
components: [check.unreferenced]
requirement: check.unreferenced/commands-in-crate@215013cd
---

Two of the 13 tests moved-tests.txt lists for check.unreferenced still run the clm binary from crates/clm/tests/unreferenced.rs: scenario_named_as_a_detector (runs clm check with the binary's own file as a detector, a case the root's Design keeps in place) and scenario_from_a_subdirectory_of_a_project_without_clm (observes the root crates/clm finds from the current directory, which the call takes as given). The requirement's last sentence says none runs the binary; backlog/two-unreferenced-tests-observe-only-the-clm-process.md asks for the spec change and blocks clm/routes-only.
