---
title: "A hung land check holds every land of the clone"
kind: idea
components: [land]
---

`clm land` runs `[land] check` under `.clm/check.lock` and waits for it with no time limit (`check-lock`, `lock-queue`). A test in that check that hangs, such as a polling wait with no deadline of its own (the cause of `bounded-test-runs`), never ends the check, so the land never ends and every other land of the clone waits behind it for the lock, with nothing that notices. `clm land --check-only` in a build place takes no lock but hangs its build the same way. `bounded-test-runs` bounds a session's own test runs and exempts both, since a land queues and the check runs as written.

`detector-timeouts` bounds `clm check`'s detectors the same way: a per-command limit with a default, killing the process group. A `[land] check_timeout` (or a `timeout` beside `check`) could do the same for the land check, reported as a failed check naming the limit.
