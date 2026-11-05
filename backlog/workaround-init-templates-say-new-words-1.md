---
title: "Scenario names an entry the template no longer writes"
kind: workaround
components: [init]
requirement: init/templates-say-new-words@46b3713f
---

`architecture-template-proof` (carried-constraints) removed the `## Enforced by clm` list from the written ARCHITECTURE.md. This requirement's scenario and text name its `humans-review-intent` line, which the template no longer holds, so that part cannot be observed as written. The test now checks the rest (status unbuilt, no replaced word in any template, the model loads); a spec change should reword the requirement.
