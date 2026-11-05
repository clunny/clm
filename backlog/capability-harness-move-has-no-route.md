---
title: "A rule that changes what capability tests see has no route to move the capability harness first"
kind: bug
components: [capability, orchestrator.triage, plugin, model]
issues: [142, 143]
---

A change can add a rule to a product component (here `web/home-screen`) that changes what the capability component's black-box tests observe, such as the opening screen. The capability component holds no requirements of its own, so `dependency-work-is-a-requirement` gives no requirement to add for the harness move and nothing for a `#### Needs` to name. The queue orders the capability after the new rule (`serving-rules-order-capabilities`), but the product component's land runs the whole suite. That suite includes the old capability tests, which now fail, and that build may not edit the capability's files (`path-ownership`). In notespeed the draft improvised a `deferred` item fixed through `clm claim capability --fix`, which neither triage nor the draft prompt describes.

Expected: CLM states a route for test-only work in the capability component that a component rule depends on, and the propose skill's builder check and the `clm draft` prompt both name it. The route must respect `path-ownership`, and must not make `#### Needs` point at a backlog item, which `requirement-needs` rejected. A draft can't file and fix an item in one session either, since a spec branch carries no code (`draft-checks-hand-edits`).
