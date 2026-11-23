---
title: "A requirement merged into another lists none of the sentences it loses"
kind: bug
components: [gate]
origin: 20261122-wave-4-orchestrator-by-the-rule
---

`gate/cuts` lists no `requirement <id> is gone` line for a requirement whose every scenario moves under its name to another requirement, reading it as merged. It also counts SHALL sentences only for a requirement both graphs hold. So a requirement that goes whole while its one scenario moves loses every SHALL sentence it held with no cut line: in wave 4's branch A (`20261122-wave-4-orchestrator-by-the-rule`), `orchestrator.run/says-component` and `orchestrator.triage/says-component` lose their promise that the prompts and messages use no retired word, and the gate lists neither; the change had to name them by hand under "Cuts the gate doesn't count", as wave 3 did for `sync/agents-md-spec-path`. Expected: a merged requirement's lost sentences are listed, for example by counting the SHALL sentences of the requirements that took its scenarios against the merged one's plus theirs at the base, or by a line `requirement <id> is merged into <id>` that asks for review.
