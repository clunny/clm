---
title: "An I/O failure while clm apply writes leaves the files already written"
kind: bug
components: [apply]
origin: 20261119-wave-1-by-the-rule
---

`apply/atomic` says a change with any violation leaves every file unchanged. When a write fails partway through the files a valid change writes (reported as `write failed` under `apply/atomic`), the files written before it stay written: nothing restores them. No test reaches the path.

Expected: either the writes are undone, or the requirement says what is left after a failed write. Needs a spec change either way, since no requirement states the path. `clm rename`'s undo has the same gap for what `clm apply` wrote on its behalf.

Found in wave 1 of phase 5 (2026-11-19).
