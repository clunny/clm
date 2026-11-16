---
status: done
resolves:
  - parallel-builds-fill-the-disk-and-fail-each-other-s-land-checks
---

## Intent

The build loop starts no more builds and drafts than the disk can hold: each step reads `clm machine`'s judgement of the disk, which gains a `room` (how many more builds free space covers once the running ones have grown to the largest build measured), and launches no more new runs than that, and none while a run stopped on a full disk is waiting (`backlog/parallel-builds-fill-the-disk-and-fail-each-other-s-land-checks`). Work held back waits as `waits for disk`, `clm build wait` wakes when the room grows, and only when nothing of the loop's is running does the human's queue ask for disk. Decision: `loop-holds-launches-on-disk`.

## Problem

During an unattended build of `check.health`, `clm land --check-only` failed twice with ENOSPC (a Playwright suite, then a VitePress build) after its checks had passed that far: the filesystem was at 196 GB used with 50 MB free, filled by the builds running beside it, and the 6.6 GB the build freed was used up again within minutes. So builds fail for reasons that have nothing to do with their code, and no build lands. `max_builds` bounds the disk only as a count (`build-places`). `clm machine` measures each build's worktree and judges the disk against the largest one (`machine-load-measured-by-clm`), but nothing acts on that judgement: the loop launches up to the free places whatever the disk says, and even a `disk` stop of one run holds back no new launch (only a second such stop's serial resume does).

## Approaches

- **The loop holds launches on `machine`'s measured room (chosen).** `machine` already has the measure and the growth arithmetic; one more JSON field, `disk.room`, gives the loop a number, and the loop is where launch decisions live (`build-loop-in-clm`).
- **The loop computes the room itself from `need`, `free_places`, `peak` and `available`.** Rejected: it copies `machine`'s growth rule into a second component, which drifts.
- **`clm run` and `clm draft` refuse when the disk is short** (the direction of the older idea `builds-run-out-of-disk-mid-check`). Not taken here: it holds the loop's rule in two more commands, and a hand-run build is one the human watches. That item stays for its other directions (naming `disk full` in a land's refusal, clearing build output).
- **A free-space threshold the human sets.** Rejected: it can't be right for every toolchain, which is why `machine` measures (`machine-load-measured-by-clm`).
- **Lower `max_builds`.** Rejected as the fix: it is a count, right for one project's size on one machine and wrong when the size or the free space changes.

## Outcome

- `clm machine --format json` gives `disk.room`: free space less the growth, over `peak`, rounded down, 0 below the growth, `null` with no `peak` (`machine/disk-room`).
- `clm build step` launches no more new builds and drafts than `disk.room`; none while `disk.room` is `null` and the disk is `critical`; none while a `disk` condition holds; a failed reading or an `unknown` disk holds nothing; an orphan's relaunch is never held. Held entries show `waits for disk` in `clm view queue` (`orchestrator.loop/plan-order`).
- `clm build wait` wakes when the room grows while work waits for disk (`orchestrator.loop/build-wait`).
- When the disk holds ready work and no run of the loop is planned or running, the human's queue says `free disk; the loop goes on once a build fits` (`orchestrator.loop/humans-queue`).

## Constraints

- The judgement stays `machine`'s; the loop reads it through `clm machine`'s Contract, as it reads every other command's.
- `max_builds` and places stay: the room only narrows the count, never widens it, and where free space can't be seen (a WSL virtual disk larger than the host's free space) the count is still the bound (`build-places`).
- The loop's existing tests' fake `clm` answers no `clm machine`, which holds nothing back, so their scenarios hold as written.

## Edge cases and workarounds

- What a land check writes outside the worktree (the shared `target/land`, a browser suite's artifacts) isn't measured, so the room isn't padded for it; a land that still runs out stops its run with cause `disk`, which then holds every new launch.
- Before any build has been measured there is no room, and the loop falls back on `machine`'s `critical` state (below 5% of the filesystem).
- `clm machine` measures worktrees at most every 10 minutes, so a build that grew since its last measure is counted at its older size; the growth still owed sets aside up to `peak` for it.

## Open questions

- The capability test `crates/clm/tests/capability/build_from_spec.rs` (`capability/build-from-spec`) drives the real `clm build step` with the real `clm machine`. Its temporary repository has no measured `peak`, so it is held back only when the filesystem is below 5% free (`critical`); on a nearly full machine its `launch` lines then go. Left as is: the test would fail then for the reason the loop is right to hold, and a run on a disk that full fails other suites too.
