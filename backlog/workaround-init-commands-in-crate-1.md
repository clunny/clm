---
title: "Seventeen listed tests stay in the root, raised as a question"
kind: workaround
components: [init]
requirement: init/commands-in-crate@130bb26b
---

Seventeen of the 100 tests moved-tests.txt lists for init still run the clm binary from crates/clm/tests/. Eleven in init.rs observe another component's command on what clm init wrote (clm check, clm hashes, clm pending, clm setup), which clm-init can neither call (check depends on init) nor run as a binary. Six in guide.rs could move, but the root's doc_readers.rs asserts crates/clm/tests/guide.rs holds the guide's clm:reads markers, and an impl/init build may not edit that root file. The requirement's last sentence says none runs the binary; backlog/seventeen-init-tests-can-t-move-into-its-crate.md asks for the spec change and the root's edit, and blocks clm/routes-only.
