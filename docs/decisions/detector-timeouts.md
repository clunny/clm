---
title: "Each detector has a timeout, default 600 seconds, and its output streams as it arrives"
date: 2026-10-27
status: accepted
---

`clm check` waited for each detector with no limit and buffered its output until it exited, so a detector blocked on a cargo lock, a network fetch or a prompt hung `clm check`, `clm land` and unattended builds with nothing on screen saying which (`backlog/detector-timeout`).

Now each `[[detectors]]` table may set `timeout`, whole seconds, default 600. A detector still running at its timeout is killed with the processes it started, reported as `<name> timed out after <n>s`, and the next detector runs. On Unix the detector gets a process group of its own and the group is killed, so a `sh -c` wrapper can't leave a child holding a lock. Its stdout and stderr are copied to `clm check`'s stderr line by line as they arrive, so a slow detector shows what it is doing.

600 seconds is long enough for a detector that compiles the project (`cargo udeps`) on a cold cache and short enough that an unattended build notices within one step. A repository that needs more sets it per detector.

Rejected:
- **One timeout for all detectors**, a top-level key. Detectors differ by orders of magnitude; a per-table key costs nothing more to read.
- **No default**, a timeout only when set. The hang this fixes comes from detectors nobody expected to hang.
- **A timeout on the whole `clm check`.** It wouldn't say which detector hung, and `run`'s rules never block.
