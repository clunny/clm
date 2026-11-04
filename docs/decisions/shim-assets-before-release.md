---
title: "A release fails when its shim names an asset the release won't build"
date: 2026-10-27
status: accepted
---

The shim (`plugin`) and the release's build matrix (`ci`) are two nodes, built separately, yet a release is only usable when every asset the shim at that commit downloads is one the release publishes. Renaming a target (`-gnu` to `-musl`) or adding one (Windows) leaves a window between the two builds landing, and a release dispatched in that window would publish a version whose shim can't find its binary.

`release-prepare.sh` therefore reads `plugin/bin/clm`'s `target=<target>` assignments and the build matrix's `target:` lines, and fails, naming the missing asset, unless every shim target is built. A release dispatched in the window refuses instead of publishing a broken version, and the check also catches a shim edited later without the matrix. The shim's builds come after the workflow's (`plugin/shim-targets` needs `ci/release`), so `main` always holds a matrix at least as wide as the shim it releases once both have landed.

Rejected:
- Keeping the old targets in the matrix during the switch (expand, then contract): a second change to remove them, for a window a refused dispatch already covers.
- A fallback in the shim from the new asset name to the old: code that is dead after the first release built with the new matrix.
- A `cargo test` cross-check of the two files: it would fail the `ci` build that changes the matrix, since that build can't edit the shim, leaving it no way to land.
