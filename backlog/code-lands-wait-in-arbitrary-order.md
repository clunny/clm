---
title: "Lands waiting for the land check go in arbitrary order, and a cheap land waits behind a slow one"
kind: idea
components: [land, config]
---

When several lands queue for a clone's check (`check-runs-one-at-a-time`, and the land lock in `land-target-lock`), each waits on an operating-system file lock, which wakes waiters in no stated order. A land that came first can be passed over, and a land whose check takes seconds waits behind one whose check takes twenty minutes. This shows up whenever `/clm:build` runs several builds that finish close together, as it does by default.

## Direction: an ordered queue with a priority lane

- **Ordered.** Lands that need the check queue first in, first out, through a ticket a land takes when it starts waiting (a file under the clone's `.clm/`, so it stays per clone, never per machine). A land that dies leaves its ticket stale, so a ticket names its process and a dead one is skipped.
- **Priority lane.** A land that is cheap goes ahead of the ordered queue. "Cheap" must be judged by what the land will run, never by language-specific facts. Candidates: the land runs only a selection of tests (`CLM_SELECTION` from `clm affected`, or the readers of checked code), or the ledger's past check times for the same command and selection say it is short. A plain cap keeps the lane from starving the ordered queue, for example at most N lane lands in a row while the ordered queue is waiting.
- Lands that run no check already take no lock (`land-target-lock`), so they need no lane.
- The land says where it is in the queue when it waits ("waiting: 2 lands ahead"), so a build's `waiting:` line and the human both see progress.

## Toolchains

Nothing here may assume cargo. The queue is about the check command, which a project defines (`[land] check`), so it applies to every project. What makes a land cheap differs by toolchain: in a compiled language (Rust, Go, C++, Java, C#) a check that needs a build costs far more than one that only runs tests already built, while in an interpreted one (Python, JavaScript without a bundle step, Ruby) the cost is mostly in the tests themselves. So the lane's test should rest on measured time or on the size of the selection, which are neutral, not on "no compile".

Related: `queued-lands-each-run-a-full-check` (batching the queue), `a-land-check-that-builds-nothing-still-waits-for-one-that-does` (lands that don't need the exclusive lock at all), `slow-land-checks-go-unnoticed` (measuring the check).
