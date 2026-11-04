---
title: "The build skill says clm next waits on settled dependencies"
kind: bug
components: [plugin]
origin: 20261104-found-by-search-and-agents
---

What happened: `plugin/skills/build/SKILL.md` step 1 tells Claude that `clm next` prints components "whose dependencies are all settled (`node-reconciliation`)". `depends_on` no longer orders builds: only a requirement's `#### Needs` and the backlog items that block it hold work back (`needs-alone-order`), and the constraint is now named `component-reconciliation`. A session following the skill may wait on, or explain the queue by, an ordering CLM no longer applies.

Expected: the step says `clm next` prints the components with pending requirements that are unclaimed and have a requirement no `#### Needs` or blocking backlog item holds back, and names `component-reconciliation` and `needs-alone-order`.

Rule involved: `needs-alone-order`, `component-reconciliation`; found while auditing the guide against the specs.
