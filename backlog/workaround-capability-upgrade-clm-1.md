---
title: "Required migration not reachable at the boundary"
kind: workaround
components: [capability]
requirement: capability/upgrade-clm@7ef3a507
---

The scenario's pending required migration can't be set up at the boundary: the shipped clm holds only advised migrations. The test reaches the refusal through a stamp naming an unknown migration (exit 4, names it), so 'points at clm upgrade' is not checked. A spec or a test hook for a required migration is needed.
