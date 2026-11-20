---
title: "run_within can report a timeout for a command that exited in time, and a sample's time may be an impossible date"
kind: bug
components: [vcs, machine]
origin: 20261119-wave-1-by-the-rule
---

Found by the second builder check of wave 1 of phase 5 (2026-11-19), read from the code.

- `crates/clm-git/src/lib.rs:164-178`: `run_within` returns the timed-out error for a command that exited within its limit when a process it started still holds a pipe and the readers haven't finished, against `vcs/bounded-runs`' first sentence, that it behaves as `run` when the command exits within the limit.
- `crates/clm-machine/src/time.rs`: an impossible date such as `2026-02-31` is read as a sample's time, so `machine/machine-history` counts a line that isn't a sample.
