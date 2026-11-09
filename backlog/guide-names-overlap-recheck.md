---
title: "The guide doesn't say a land re-checks only the overlap, or name CLM_LAND_DIR and land-checks-stay-fast"
kind: deferred
components: [init]
origin: 20261108-toolchain-neutral-land-builds
---

Once `land/overlap-recheck` and `land/land-dir-env` are built and released, add to the guide's `ci-and-deployment` page: that a land whose head was checked whole runs only the tests the branch and `main`'s new commits can both affect, and none when there are none; the `re-check:` line and the `landed` record's `recheck`; `CLM_LAND_DIR`; and what `land-checks-stay-fast` asks of a project's tests. Saying it before the build would describe behaviour no released `clm` has (`guide-from-docs`).
