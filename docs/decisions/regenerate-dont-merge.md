---
title: "Derived files are regenerated, never merged"
date: 2026-11-13
status: accepted
---

`ARCHITECTURE.md`'s entry `hash-based-staleness` states this in short (`human-layer-entries-stay-short`); the rule in full:

Derived files SHALL be regenerated, never merged. Indexes, registries and the applied `specs/` are rebuilt from their sources (`derived-specs-fingerprint`).

**Scenario: conflicting derived file**
- GIVEN two branches that each change a derived file
- WHEN the second is brought up to `main`
- THEN the file is regenerated from the merged sources, not merged by hand
