---
status: proposed
resolves:
  - guard-treats-a-piped-clm-land-help-as-a-land
---

# Guard lets a piped `clm land --help` through

## Intent

Designing: `clm land --help | head` lands nothing, and the guard refuses it.
