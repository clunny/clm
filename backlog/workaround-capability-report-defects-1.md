---
title: "Scenario misfire not observable at the boundary"
kind: workaround
components: [capability]
requirement: capability/report-defects@b238d613
---

The scenario's GIVEN, a session where a check misfired, lies beyond the clm binary's boundary: no command can observe that a check misfired, so the capability test only describes the misfire in the report text it files with `clm defect`. The spec-auditor found the GIVEN isn't set up.
