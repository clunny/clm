---
title: "Derived files are regenerated, never merged"
date: 2026-11-13
status: accepted
---

`ARCHITECTURE.md` stated this as the entry `regenerate-dont-merge` until 2026-11-13, when it was merged into `hash-based-staleness` (`human-layer-entries-stay-short`). The text still holds:

Derived files SHALL be regenerated, never merged. Indexes, registries and the applied `specs/` are rebuilt from their sources (`derived-specs-fingerprint`).

**Scenario: conflicting derived file**
- GIVEN two branches that each change a derived file
- WHEN the second is brought up to `main`
- THEN the file is regenerated from the merged sources, not merged by hand
