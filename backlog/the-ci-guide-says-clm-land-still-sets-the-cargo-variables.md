---
title: "The ci-and-deployment guide page says clm land still sets the cargo variables and that a caller's CARGO_TARGET_DIR skips the lock"
kind: bug
components: [land, ci]
---

`impl/land` built `land/land-dir-env`, `land/shared-land-target` and `land/land-target-lock` as they now read: `clm land` sets `CLM_LAND_DIR` for every land that runs `[land] check`, sets and reads neither `CARGO_TARGET_DIR` nor `CARGO_INCREMENTAL`, and takes the land lock and the land tree whether or not the caller set `CARGO_TARGET_DIR`. `docs/guide/ci-and-deployment.md` (around line 102 and the sentence after the shell example) still says a land that takes no lock, as when you set `CARGO_TARGET_DIR` yourself, runs the check with `CLM_LAND_DIR` unset, and that `clm land` still sets the two cargo variables for now. Both sentences are false. Say instead that `--check-only` alone runs with `CLM_LAND_DIR` unset, and that `clm land` sets no toolchain variable and leaves the caller's `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` as they are.

A docs-only edit that changes no requirement takes the small-change route; it was left out of `impl/land` because a guide edit makes a code branch mixed (`gate/classify`).
