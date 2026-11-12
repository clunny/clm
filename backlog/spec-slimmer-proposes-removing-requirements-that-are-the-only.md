---
title: "spec-slimmer proposes removing requirements that are the only proof of a decision or an Enforced-by entry"
kind: bug
components: [plugin.agents]
origin: 20261111-slim-flagged-specs
---

Step: /clm:health launches the spec-slimmer for each flagged component, and /clm:propose then acts on its proposed cuts.
What happened: the slimmer proposed deleting source-scan requirements (a crate's shared-plumbing and git-through-layer rules) as "implementation detail, not behaviour". Those requirements are named in an ARCHITECTURE.md `#### Enforced by` block, or are the per-crate tests a decision says prove it. Removing them would leave the constraint or decision unproven. The proposal reached the human as a recommended cut before the propose session checked it.
Expected: the slimmer checks whether an `#### Enforced by` block, a decision or another component's `#### Needs` names a requirement before proposing to remove or merge it, and says so when one does.
Rules involved: proven-by-tests, deterministic-enforcement, health-advises.
