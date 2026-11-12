---
title: "Propose skill doesn't tell an archive-in-spec change to record the code work its Outcome promises"
kind: bug
components: [plugin.propose]
origin: 20261111-slim-flagged-specs
---

Step: /clm:propose, step 9, for a change whose deltas add, modify and remove no requirement (archived in its own spec branch, archive-in-spec-pr).
What happened: an earlier Design-only change said in its Outcome that "the build removes" some code nothing calls. No requirement changed, so no component went pending, no build ran, and the code stayed. Nothing recorded the promised work, and it surfaced only when a later slimming pass found the same code.
Expected: the skill tells a change whose deltas modify no requirement to record any code work it relies on as a `kind: deferred` item (as its Harness-only section already does for a Design edit), and the builder check asks whether the Outcome promises code work no requirement makes pending.
Rules involved: node-reconciliation, dependency-work-is-a-requirement, deferred-work-resolves-in-code.
