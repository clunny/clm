---
title: "The clone's shared land and verify build caches grow without bound"
kind: bug
components: [land, verify]
---

What happened: during a /clm:build session, parallel builds' land checks failed with "no space left on device". Measuring the clone found the space held mostly by CLM's own shared build caches, not by work: the land check's build directory (13 GB) and the build output in `clm verify`'s checkout (30 GB, 26 GB of it in the debug profile). Both had grown across many lands and verify runs, because the toolchain's build cache keeps every old artifact and nothing in CLM ever prunes or clears either directory. The human had to delete them by hand, and only after builds had already failed.

Expected: CLM bounds the caches it creates. For example, it clears or prunes them when they exceed a size or age, through the project's own clean command or a declared output path, or `clm wrap-up` or `clm health` reports their size and offers to clear them, so they never silently fill the disk.

Related: backlog/builds-run-out-of-disk-mid-check covers per-build worktree output and the missing free-space check before a build or land starts. This report covers the clone-wide caches, which keep growing even when only one build runs at a time.
