---
title: "Scenario names an entry the template no longer writes"
kind: workaround
components: [init]
requirement: init/template-says-pull-request@f22d765f
---

`architecture-template-proof` (carried-constraints) removed the `## Enforced by clm` list from the written ARCHITECTURE.md. This requirement's scenario names that list's line on the single source of truth, which the template no longer holds, so the scenario cannot be observed as written. The test now checks what remains true (no template calls a host's pull request a page); a spec change should reword the scenario.
