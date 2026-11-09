---
status: proposed
resolves: [verify-bisect-blames-commits-it-could-not-test]
---

## Intent

`clm verify`'s bisect skips commits whose check cannot run, rather than counting them red, and records `culprit unknown` when no commit in the range can be tested.
