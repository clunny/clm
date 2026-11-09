---
title: "clm land still sets CARGO_TARGET_DIR and CARGO_INCREMENTAL for every project's check"
kind: deferred
components: [land, ci, init]
origin: 20261108-toolchain-neutral-land-builds
---

`20261108-toolchain-neutral-land-builds` gave every land check in the land tree `CLM_LAND_DIR` (`land/land-dir-env`), the first step of moving cargo's settings out of CLM and into each project's check script. `clm land` still sets `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` itself, and ties the land lock and the land tree to a caller's `CARGO_TARGET_DIR` (`shared-land-target`, `land-target-lock`, `fixed-land-tree`).

The next change, once a release carrying `upgrade/migration-land-dir` has shipped and `ci/land-dir` is current, goes by `expand-contract`: the starter `scripts/clm-check.sh` that `clm init --lang rust` writes exports `CARGO_TARGET_DIR` from `CLM_LAND_DIR`, with an `apply` migration whose `old` is the script the latest release writes (`upgrades-carry-migrations`, `clm/upgrade-corpus`); then `clm land` stops setting the cargo variables; the land lock and the land tree follow whether the land runs `[land] check` at all, not a caller's `CARGO_TARGET_DIR`; `shared-land-target`, `land-target-lock`, `land-target-cap` and `land`'s Design say so; and the guide's `ci-and-deployment` page names `CLM_LAND_DIR` (`guide-from-docs`).
