---
title: "No gate test has a rename rewrite a marker in a code file"
kind: bug
components: [gate]
origin: 20261113-phase-2-delete
---

`gate/classify`'s scenario "rename branch" says a branch holding the committed result of `clm rename` prints `spec`, and its code files count as the replay's. The fixture in `crates/clm-gate/tests/rename.rs` holds no file with a marker, so the rename's diff changes no code file and the path that accepts the replay's code files never runs with one.

The requirement was flagged while `rename/marker-rewrite` was unbuilt (`workaround-gate-classify-1`, deleted in `20261113-phase-2-delete`). It is built now, so the gap can close: give the fixture a test file citing the renamed requirement, assert the rename rewrote it, and assert the gate prints `spec` for that branch and `mixed` for the same branch with an extra code edit.
