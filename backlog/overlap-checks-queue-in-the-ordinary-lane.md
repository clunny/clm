---
title: "Lands that run an overlap check queue in the ordinary lane behind full checks"
kind: deferred
components: [land]
origin: 20261108-toolchain-neutral-land-builds
---

A land that runs only the overlap of its branch and `main`'s new commits (`land/overlap-recheck`) takes an `ordinary` ticket for `land.lock` and `check.lock`, like a land of unchecked code, so a small overlap check can wait behind full checks; only readers of checked code take the `priority` lane (`lock-queue`, `ordered-lock-queue`).

Decide once the ledger has measurements: `ledger/landed-recheck` records each land's `recheck` and `recheck_files`, and `landed` records keep check times. When enough lands have run, read how often lands run an overlap, how many files it selects and how long those checks take against full ones, and whether overlap lands wait long behind them. Then decide whether overlap lands take the `priority` lane, perhaps only below a size, or stay `ordinary`. The human asked for this to wait for measurements (2026-11-08).
