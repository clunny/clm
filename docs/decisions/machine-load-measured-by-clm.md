---
title: "Machine load is read and judged by clm machine, the disk against the builds' measured peak, and charted by the pane"
date: 2026-11-12
status: accepted
---

The CLM pane shows the machine's load so a person watching builds can see trouble coming: CPU queueing beyond the cores for a sustained time, available memory running out, or the disk running short of what the builds still need. The Capabilities tab it had goes, and with it the idea of an Architecture tab beside it: the pane is for watching builds, and the capability and architecture review stays with `clm capabilities`, `clm architecture` and the web app.

- **`clm` reads and judges; the pane draws.** The pane's hooks module has no Node.js and can only run commands, so reading `/proc` or `sysctl` itself would be per-platform parsing in TypeScript. More to the point, only `clm` knows the clone's build places and `max_builds`, which the disk judgement needs, and the same judgement can later stop `clm run` from starting a build the disk can't hold (`builds-run-out-of-disk-mid-check`). So `clm machine` (component `machine`) takes the readings and their states, and the pane only keeps a history and draws it, as it does for the views (`views-rendered-by-clm`).
- **CPU is the load average over the cores**, judged on the 5-minute one: the operating system's own smoothing, so a sustained load flags and a spike doesn't, without the command sampling over time. A build keeps it near 1; amber from 1.25 and rose from 2 flag queueing, not a busy machine.
- **Memory is the available memory**, not the free memory: what the system can still hand out, page cache it can reclaim included. Free memory reads near zero whenever the cache is full; allocations fail only when available memory runs out.
- **Disk is judged against what the builds have been measured to use**, not a size the human sets: a build's worktree is 6 to 9 GB in this repository and may be little in a Python or Go project, so only a measurement holds for any toolchain. `clm machine` measures each counting build's worktree at most every 10 minutes, keeps the largest in a clone-local record, and turns rose when free space can't cover the running builds growing to that size, amber when it also can't cover a build in every free place. Measuring only at land would learn slowly and miss builds that fail; inferring use from drops in free space is noisy whenever anything else writes.
- **A history, not a gauge.** The pane samples every 10 seconds and charts the last 30 minutes, since the trend says more than one reading: a disk that is falling steadily is trouble before it is red.

Under WSL the filesystem is a virtual disk whose free space can exceed the Windows host's, so the disk can read green while the host is full; nothing inside the system sees the host's disk, as `build-places` already notes.
