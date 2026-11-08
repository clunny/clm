---
status: proposed
resolves:
  - backlog/overlapping-lands-fails-under-the-full-suite-s-load-but-passes
---

## Intent

A land that fails to fetch `origin` only because another land in the same clone moved a remote-tracking ref under it is not offline: it tries the fetch again, and says it is offline only when `origin` can't be reached.
