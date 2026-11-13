---
title: "Adopters' architecture entries carry no label, and nothing asks for one"
kind: idea
components: [init, upgrade, model]
origin: 20261113-human-layer-rewrite
---

`deterministic-enforcement` is a carried `method` constraint, and since 2026-11-13 it asks every rule to say whether it is checked, measured or a principle (`three-kinds-of-rule`). CLM's own `ARCHITECTURE.md` writes the label as the first word of each entry's statement, which the model doesn't read.

The `ARCHITECTURE.md` that `clm init` writes says nothing of labels, the entries that projects set up by a published release already hold have none, and no check reads one. Whether the label becomes something the model reads, with the template and an advised migration to match (`upgrades-carry-migrations`), belongs with the plan's phase that adds the block for measured entries (`backlog/a-passing-check-proves-less-than-the-spec-claims`).
