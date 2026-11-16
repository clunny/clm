---
title: "clm land leaves its /tmp/land-bin copy behind"
kind: bug
components: [land]
---

Step: `clm land` / `clm land --check-only` copies the clm binary into a /tmp/land-bin-* directory before running the checks.
What happened: such directories are left behind after the land ends. One dates from two days before any land currently running, so it was never removed; each holds about 20 MB. A failing land (here because the disk was full) is the likely case.
Expected: the land removes its temporary binary copy on every exit path, success or failure.
Rule: no-dead-code in spirit; leaked temp state also adds to the disk pressure already reported for parallel builds.
