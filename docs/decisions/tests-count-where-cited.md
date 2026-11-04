---
title: "A test counts toward the nodes its markers cite, not the node whose paths hold it"
date: 2026-10-25
status: accepted
---

`check/node-size` measures what a session reads to build a node (`node-size-budgets`). Code was counted by `paths`. That gave the root node 122k tokens of which 103k were `crates/clm/tests/*.rs`: integration tests that run the built `clm` binary and prove other nodes' requirements (113 markers cite `orchestrator`, 88 `ci`, 46 `plugin`). Those files live in the root crate only because Cargo exposes the binary (`CARGO_BIN_EXE_clm`) to that package's tests alone, and the builds that read and edit them are the cited nodes', not the root's. The root's number said nothing about the root, and the cited nodes' numbers left out their own test suites.

Now a tracked file with `req:` markers counts toward every node whose requirements it cites, and not toward the node whose `paths` hold it unless they cite it too. A file with no markers counts toward its `paths` owner. The marker scan `check` already runs supplies the attribution, so the measure stays deterministic and needs no new convention.

Rejected:
- **Moving the test files into the crates they prove.** The binary isn't available to another package's tests; `assert_cmd`'s workspace lookup finds it only when the whole workspace was built first, so `cargo test -p <crate>` would break. And `paths` are disjoint for a reason: two builds never edit the same files.
- **Carving the test files out of the root's `paths` into the other nodes'.** The root's globs can't express an exclusion, each node would list files inside another crate, and the files are still read by their cited nodes either way.
- **Counting a mixed file toward its `paths` owner only.** A file that cites two nodes is read by both builds; counting it once in the wrong place is what the old measure did.
