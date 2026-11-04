---
title: "Needs model/content-hash built first, but names no Needs"
kind: workaround
components: [apply]
requirement: apply/stale-base@6b3410c6
---

Scenario "serves edited under a delta that carries a block" expects `apply/stale-base` to report a delta stale after an edit to only the rule's `#### Serves` block. That edit moves a requirement's hash only once `model/content-hash` (two-part hash, `serves-in-the-base`) is built; on `main` it is still `behind` (cited at db26ea3e, current 47dcb5e3), so the model's hash still leaves the block out entirely and the scenario can't be observed (a test written for it fails: the hash before and after the Serves edit is equal). The requirement lacks `#### Needs model/content-hash`, so `clm next` handed out `apply` before `model`. Left unbuilt; `backlog/apply-stale-base-needs-model-content-hash.md` asks a spec branch for the Needs.
