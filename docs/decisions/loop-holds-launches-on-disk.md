---
title: "The build loop starts no more builds than the disk's measured room"
date: 2026-11-15
status: accepted
---

The build loop (`orchestrator.loop`) reads `clm machine --format json` in each step's snapshot and starts no more new builds and drafts than its `disk.room`: how many more builds free space covers once the running builds have grown to the largest build the clone has measured (`machine-load-measured-by-clm`). It starts none while a `disk` condition holds, none while there is no measurement yet and the disk reads `critical`, and it holds nothing back when the reading fails or the disk is `unknown`. Work it holds back waits with the reason `waits for disk`, `clm build wait` wakes when the room grows, and when nothing of the loop's is running, so no land will free space, the human's queue asks for disk.

**Why.** On 2026-11-15 parallel builds filled a 196 GB filesystem to its last 50 MB, and a build's `clm land --check-only` failed twice with ENOSPC in suites that had nothing to do with its code; clearing 6.6 GB was used up again within minutes by the builds beside it. `build-places` bounds the disk by a count of places, `max_builds`, which can't know how much a build of a given project takes. `machine-load-measured-by-clm` already measures it and named this use: the same judgement that colours the pane can hold back a build the disk can't hold.

**Why the loop, not `clm run`.** The loop decides what to launch (`build-loop-in-clm`); a refusal in `clm run` and `clm draft` would hold the same rule in two more commands, and a session that runs `clm run` by hand is one the human is watching. The room is `machine`'s to compute, so the loop holds no copy of the growth arithmetic.

**Why not padding for the land check.** A land check writes beyond the worktree (the shared `target/land`, a browser suite's artifacts). The land directory is one, whatever the number of builds, and already takes its space; the artifacts are small beside a build's output. Padding the room for them would be a guess the measurement exists to avoid (`yagni`); if a land still runs out, the run stops with cause `disk` and the condition holds every new launch until the human clears it.

`build-places`' reason for a count still stands where free space can't be seen (a WSL virtual disk larger than the host's free space): the count stays, and the room only narrows it.
