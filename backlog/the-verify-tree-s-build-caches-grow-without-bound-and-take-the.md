---
title: "The verify tree's build caches grow without bound and take the build loop's room"
kind: bug
components: [verify]
---

Step: `clm verify` (run by the build loop's `run verify clm verify --due`), then the loop's room check for more builds.

What happened: after a day of verifies, `.clm/verify-tree/target` held about 72G: one cargo target directory per mutation job (`mutate-job-1` to `mutate-job-5`, 10-12G each) plus `llvm-cov-target` (15G). Nothing removed or bounded them between runs. The loop sets aside about 21G per running build before counting free room, so these caches cost it roughly three build places: the queue showed builds as "waits for disk" until the human was asked and the caches were deleted by hand, after which two more builds launched at once.

Expected: verify keeps its caches within a bound (or shares one target across mutation jobs, or removes the per-job targets after a run), or the loop's room check names the verify tree's caches as reclaimable and says so in the queue, so the human isn't left to find them.

Rule involved: `disk-room` (machine), and verify's own tree.
